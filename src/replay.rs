use crate::AggregateRoot;
use crate::event_store::{StreamHistory, StreamRevision};
use crate::snapshot::{AggregateSnapshot, SnapshotUse};

/// **English:** Aggregate hooks for reconstructing state from stored events or a snapshot.
/// Implementations should apply historical events without recording new pending events.
/// The replay helpers do not clear queues, check aggregate identity, publish events, or catch
/// panics from these hooks. Error and panic conditions are defined by each implementation.
///
/// **日本語:** 保存イベントまたはスナップショットから状態を復元するための集約フックです。
/// 実装では履歴イベントの適用で新たな保留イベントを記録しないようにします。
/// 再生ヘルパーはキューの消去、集約識別子の確認、イベント発行、フックのパニック捕捉をしません。
/// エラーとパニックの条件は各実装が定めます。
pub trait AggregateReplay: AggregateRoot {
    /// **English:** Failure to create the pre-event state.
    ///
    /// **日本語:** イベント適用前の状態の作成失敗です。
    type InitializationError;
    /// **English:** Failure to apply a historical event.
    ///
    /// **日本語:** 履歴イベントの適用失敗です。
    type ApplicationError;
    /// **English:** State representation consumed when restoring a snapshot.
    ///
    /// **日本語:** スナップショット復元時に消費する状態表現です。
    type Snapshot;
    /// **English:** Failure to classify a snapshot, distinct from choosing full replay.
    ///
    /// **日本語:** スナップショット分類の失敗です。全件再生を選ぶこととは区別されます。
    type SnapshotValidationError;
    /// **English:** Failure to construct aggregate state from an accepted snapshot.
    ///
    /// **日本語:** 採用したスナップショットからの集約状態構築失敗です。
    type SnapshotStateError;

    /// **English:** Creates state for `id` before any historical events are applied.
    /// Returns an implementation-defined error if initialization fails.
    ///
    /// **日本語:** 履歴イベント適用前の `id` の状態を作成します。
    /// 初期化に失敗した場合は実装定義のエラーを返します。
    fn initial_state(id: &Self::Id) -> Result<Self, Self::InitializationError>
    where
        Self: Sized;

    /// **English:** Updates state using one borrowed historical event, without emitting it again.
    /// Returns an implementation-defined error on failure; mutations before that error are not undone.
    ///
    /// **日本語:** 借用した履歴イベント 1 件で状態を更新し、そのイベントを再発行しません。
    /// 失敗時は実装定義のエラーを返します。それ以前の状態変更は元に戻されません。
    fn apply_event(&mut self, event: &Self::Event) -> Result<(), Self::ApplicationError>;

    /// **English:** Classifies `snapshot` against the complete history's `stream_revision`.
    /// Return [`SnapshotUse::Applicable`] to restore it, or another classification to replay all events.
    /// An error aborts replay rather than falling back. Compatibility and staleness policy belong here.
    ///
    /// **日本語:** 完全な履歴の `stream_revision` に照らして `snapshot` を分類します。
    /// 復元するなら [`SnapshotUse::Applicable`]、全件再生するなら他の分類を返します。
    /// エラー時はフォールバックせず再生を中断します。互換性と古さの方針をここで定めます。
    fn classify_snapshot(
        snapshot: &AggregateSnapshot<Self::Snapshot>,
        stream_revision: StreamRevision,
    ) -> Result<SnapshotUse, Self::SnapshotValidationError>;

    /// **English:** Consumes an accepted snapshot to construct the aggregate for `id`.
    /// Returns an implementation-defined state error; replay does not retry from the initial state.
    ///
    /// **日本語:** 採用済みスナップショットを消費し、`id` の集約を構築します。
    /// 失敗時は実装定義の状態エラーを返し、初期状態からの再試行は行いません。
    fn from_snapshot(
        id: &Self::Id,
        snapshot: AggregateSnapshot<Self::Snapshot>,
    ) -> Result<Self, Self::SnapshotStateError>
    where
        Self: Sized;
}

/// **English:** Reconstructed aggregate paired with the history's final revision.
/// This revision can be used as [`crate::ExpectedRevision::Revision`] for a subsequent append.
///
/// **日本語:** 復元した集約と履歴の最終リビジョンです。
/// このリビジョンは次の追加の [`crate::ExpectedRevision::Revision`] として利用できます。
#[derive(Debug, Eq, PartialEq)]
pub struct RehydratedAggregate<A> {
    /// **English:** Aggregate after all selected historical events were applied.
    ///
    /// **日本語:** 選択された履歴イベントをすべて適用した後の集約です。
    pub aggregate: A,
    /// **English:** Final revision of the complete input history, including when no tail events remain.
    ///
    /// **日本語:** 後続イベントが残っていない場合も含め、入力された完全な履歴の最終リビジョンです。
    pub revision: StreamRevision,
}

/// **English:** First failure encountered during full-history reconstruction.
///
/// **日本語:** 全履歴からの復元で最初に発生した失敗です。
#[derive(Debug, Eq, PartialEq)]
pub enum ReplayError<I, P> {
    /// **English:** Creating the initial aggregate state failed.
    ///
    /// **日本語:** 集約の初期状態の作成に失敗しました。
    Initialization(I),
    /// **English:** Applying a historical event failed.
    ///
    /// **日本語:** 履歴イベントの適用に失敗しました。
    Application(P),
}

/// **English:** First failure during snapshot-assisted reconstruction; errors do not trigger fallback.
///
/// **日本語:** スナップショットを用いた復元で最初に発生した失敗です。エラー時はフォールバックしません。
#[derive(Debug, Eq, PartialEq)]
pub enum SnapshotReplayError<I, P, V, S> {
    /// **English:** Creating initial state for full replay failed.
    ///
    /// **日本語:** 全件再生のための初期状態の作成に失敗しました。
    Initialization(I),
    /// **English:** The snapshot classification hook returned an error.
    ///
    /// **日本語:** スナップショット分類フックがエラーを返しました。
    SnapshotValidation(V),
    /// **English:** Restoring the accepted snapshot failed.
    ///
    /// **日本語:** 採用したスナップショットの復元に失敗しました。
    SnapshotState(S),
    /// **English:** An applicable snapshot's tail could not be selected by [`StreamHistory::events_after`].
    /// This includes a snapshot ahead of history or an unrepresentable next index.
    ///
    /// **日本語:** 採用可能なスナップショットの後続部分を [`StreamHistory::events_after`] で選べませんでした。
    /// 履歴より先のスナップショットや、表現できない次の添字が該当します。
    SnapshotRevision {
        /// **English:** Revision carried by the accepted snapshot.
        ///
        /// **日本語:** 採用したスナップショットが保持するリビジョンです。
        snapshot_revision: StreamRevision,
        /// **English:** Final revision of the supplied history.
        ///
        /// **日本語:** 指定された履歴の最終リビジョンです。
        history_revision: StreamRevision,
    },
    /// **English:** Applying an event from the selected history failed.
    ///
    /// **日本語:** 選択された履歴のイベント適用に失敗しました。
    Application(P),
}

/// **English:** Initializes `A` for `id` and applies every event in `history` in stored order.
/// Returns the reconstructed aggregate and history revision. The first hook error is wrapped in
/// [`ReplayError`]; the local aggregate is dropped on error. No storage or publishing is performed,
/// and hook panics propagate. The caller must supply history belonging to `id`.
///
/// **日本語:** `id` の `A` を初期化し、`history` の全イベントを保存順に適用します。
/// 復元した集約と履歴リビジョンを返します。最初のフックエラーを [`ReplayError`] で包み、
/// エラー時はローカルな集約を破棄します。保存や発行は行わず、フックのパニックは伝播します。
/// 呼び出し側は `id` に属する履歴を指定する必要があります。
///
/// **English:** This counter restores two historical increments without queuing new events.
///
/// **日本語:** このカウンターは新規イベントを保留せず、履歴の加算 2 件を復元します。
///
/// ```rust
/// use std::convert::Infallible;
/// use ddd_toolkit_core::{
///     AggregateReplay, AggregateRoot, AggregateSnapshot, DomainEvent, Entity,
///     SnapshotUse, StreamHistory, StreamRevision, replay_aggregate,
/// };
/// struct Increment(u64);
/// impl DomainEvent for Increment {}
/// struct Counter { id: u64, total: u64, pending: Vec<Increment> }
/// impl Entity for Counter {
///     type Id = u64;
///     fn id(&self) -> &u64 { &self.id }
/// }
/// impl AggregateRoot for Counter {
///     type Event = Increment;
///     fn record_event(&mut self, event: Increment) { self.pending.push(event); }
///     fn pull_events(&mut self) -> Vec<Increment> { std::mem::take(&mut self.pending) }
/// }
/// impl AggregateReplay for Counter {
///     type InitializationError = Infallible;
///     type ApplicationError = &'static str;
///     type Snapshot = u64;
///     type SnapshotValidationError = Infallible;
///     type SnapshotStateError = Infallible;
///     fn initial_state(id: &u64) -> Result<Self, Infallible> {
///         Ok(Self { id: *id, total: 0, pending: vec![] })
///     }
///     fn apply_event(&mut self, event: &Increment) -> Result<(), &'static str> {
///         self.total = self.total.checked_add(event.0).ok_or("counter overflow")?;
///         Ok(())
///     }
///     fn classify_snapshot(_: &AggregateSnapshot<u64>, _: StreamRevision)
///         -> Result<SnapshotUse, Infallible> { Ok(SnapshotUse::Applicable) }
///     fn from_snapshot(id: &u64, snapshot: AggregateSnapshot<u64>) -> Result<Self, Infallible> {
///         Ok(Self { id: *id, total: snapshot.into_parts().0, pending: vec![] })
///     }
/// }
/// let history = StreamHistory::try_new(vec![Increment(2), Increment(3)], StreamRevision::new(1))
///     .unwrap();
/// let mut restored = replay_aggregate::<Counter>(&7, &history).unwrap();
/// assert_eq!(restored.aggregate.total, 5);
/// assert_eq!(restored.revision, StreamRevision::new(1));
/// assert!(restored.aggregate.pull_events().is_empty());
/// ```
pub fn replay_aggregate<A>(
    id: &A::Id,
    history: &StreamHistory<A::Event>,
) -> Result<RehydratedAggregate<A>, ReplayError<A::InitializationError, A::ApplicationError>>
where
    A: AggregateReplay,
{
    let mut aggregate = A::initial_state(id).map_err(ReplayError::Initialization)?;
    for event in history.events() {
        aggregate
            .apply_event(event)
            .map_err(ReplayError::Application)?;
    }

    Ok(RehydratedAggregate {
        aggregate,
        revision: history.revision(),
    })
}

/// **English:** Reconstructs `A` using an optional snapshot and a complete `history` for `id`.
/// With no snapshot, or a [`SnapshotUse::Stale`] or [`SnapshotUse::Inapplicable`] classification,
/// initializes state and replays all events. For [`SnapshotUse::Applicable`], selects events strictly
/// after the snapshot revision, restores state, then applies that tail in order. A snapshot at the
/// history's final revision requires no event applications.
///
/// Classification errors abort; an invalid tail yields [`SnapshotReplayError::SnapshotRevision`]
/// before state restoration. Initialization, restoration, and application errors are wrapped in
/// their corresponding variants with no fallback or retry. On failure the local state is dropped.
/// Returns the complete history's revision on success. No storage, publishing, identity checking,
/// or panic catching is performed; the caller must supply matching history and snapshot data.
///
/// **日本語:** `id` の完全な `history` と任意のスナップショットから `A` を復元します。
/// スナップショットがない場合や、分類が [`SnapshotUse::Stale`] または
/// [`SnapshotUse::Inapplicable`] の場合は初期状態から全件再生します。
/// [`SnapshotUse::Applicable`] ならスナップショットのリビジョンより後のイベントを選び、
/// 状態を復元して後続イベントを順に適用します。履歴の最終リビジョンと一致すれば適用は不要です。
///
/// 分類エラーは処理を中断し、後続部分を選べなければ状態復元前に
/// [`SnapshotReplayError::SnapshotRevision`] を返します。初期化・復元・適用のエラーは
/// 対応するバリアントで包み、フォールバックや再試行はしません。失敗時はローカル状態を破棄します。
/// 成功時は完全な履歴のリビジョンを返します。保存、発行、識別子検証、パニック捕捉は行いません。
/// 呼び出し側は対応する履歴とスナップショットを指定する必要があります。
#[allow(clippy::type_complexity)]
pub fn replay_with_optional_snapshot<A>(
    id: &A::Id,
    history: &StreamHistory<A::Event>,
    snapshot: Option<AggregateSnapshot<A::Snapshot>>,
) -> Result<
    RehydratedAggregate<A>,
    SnapshotReplayError<
        A::InitializationError,
        A::ApplicationError,
        A::SnapshotValidationError,
        A::SnapshotStateError,
    >,
>
where
    A: AggregateReplay,
{
    let (mut aggregate, events) = match snapshot {
        None => (
            A::initial_state(id).map_err(SnapshotReplayError::Initialization)?,
            history.events(),
        ),
        Some(snapshot) => {
            let snapshot_use = A::classify_snapshot(&snapshot, history.revision())
                .map_err(SnapshotReplayError::SnapshotValidation)?;
            match snapshot_use {
                SnapshotUse::Applicable => {
                    let revision = snapshot.revision();
                    let events = match history.events_after(revision) {
                        Some(events) => events,
                        None => {
                            return Err(SnapshotReplayError::SnapshotRevision {
                                snapshot_revision: revision,
                                history_revision: history.revision(),
                            });
                        }
                    };
                    (
                        A::from_snapshot(id, snapshot)
                            .map_err(SnapshotReplayError::SnapshotState)?,
                        events,
                    )
                }
                SnapshotUse::Stale | SnapshotUse::Inapplicable => (
                    A::initial_state(id).map_err(SnapshotReplayError::Initialization)?,
                    history.events(),
                ),
            }
        }
    };

    for event in events {
        aggregate
            .apply_event(event)
            .map_err(SnapshotReplayError::Application)?;
    }

    Ok(RehydratedAggregate {
        aggregate,
        revision: history.revision(),
    })
}
