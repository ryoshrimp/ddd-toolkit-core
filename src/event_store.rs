use crate::{AggregateRoot, Entity};

/// **English:** Zero-based event position; the first event in a stream has revision `0`.
///
/// **日本語:** ゼロ始まりのイベント位置です。ストリームの最初のイベントはリビジョン `0` です。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StreamRevision(u64);

impl StreamRevision {
    /// **English:** Wraps `value` without checking whether a corresponding stream exists.
    ///
    /// **日本語:** 対応するストリームの存在を確認せず `value` をラップします。
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// **English:** Returns the zero-based event position.
    ///
    /// **日本語:** ゼロ始まりのイベント位置を返します。
    pub const fn value(self) -> u64 {
        self.0
    }

    /// **English:** Returns the next revision, or `None` at `u64::MAX`.
    ///
    /// **日本語:** 次のリビジョンを返します。`u64::MAX` では `None` を返します。
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// **English:** Absence of a stream, distinct from an existing stream at revision `0`.
///
/// **日本語:** ストリームの不在です。リビジョン `0` の既存ストリームとは区別されます。
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct NoStream;

/// **English:** Stream state required by a caller when appending events.
///
/// **日本語:** イベント追加時に呼び出し側が要求するストリーム状態です。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpectedRevision {
    /// **English:** Requires that no stream exists for the identifier.
    ///
    /// **日本語:** 識別子に対応するストリームの不在を要求します。
    NoStream(NoStream),
    /// **English:** Requires a stream ending at this revision.
    ///
    /// **日本語:** このリビジョンで終わるストリームを要求します。
    Revision(StreamRevision),
}

/// **English:** Stream state observed by an adapter during a revision conflict.
///
/// **日本語:** リビジョン競合時にアダプターが観測したストリーム状態です。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurrentRevision {
    /// **English:** No stream exists for the identifier.
    ///
    /// **日本語:** 識別子に対応するストリームが存在しません。
    NoStream(NoStream),
    /// **English:** The existing stream ends at this revision.
    ///
    /// **日本語:** 既存ストリームの最終リビジョンです。
    Revision(StreamRevision),
}

/// **English:** Non-empty, complete history in replay order starting at revision `0`.
/// Construction checks event count against the final revision, not event contents or identity.
///
/// **日本語:** リビジョン `0` から始まる再生順の空でない完全な履歴です。
/// 構築時にはイベント数と最終リビジョンを照合し、内容や識別子は検証しません。
///
/// ```rust
/// use ddd_toolkit_core::{StreamHistory, StreamRevision};
/// let history = StreamHistory::try_new(vec!["created", "updated"], StreamRevision::new(1))?;
/// assert_eq!(history.events_after(StreamRevision::new(0)), Some(&["updated"][..]));
/// assert_eq!(history.events_after(StreamRevision::new(1)), Some(&[][..]));
/// # Ok::<(), ddd_toolkit_core::StreamHistoryError>(())
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct StreamHistory<E> {
    events: Vec<E>,
    revision: StreamRevision,
}

impl<E> StreamHistory<E> {
    /// **English:** Takes ownership of events and checks that `revision` equals their count minus one.
    /// Returns [`StreamHistoryError::Empty`] for no events, [`StreamHistoryError::TooManyEvents`]
    /// if the count cannot fit in `u64`, or [`StreamHistoryError::RevisionMismatch`] for a mismatch.
    ///
    /// **日本語:** イベントを所有し、`revision` がイベント数から 1 を引いた値か確認します。
    /// 空なら [`StreamHistoryError::Empty`]、件数が `u64` に収まらなければ
    /// [`StreamHistoryError::TooManyEvents`]、不一致なら [`StreamHistoryError::RevisionMismatch`] を返します。
    pub fn try_new(events: Vec<E>, revision: StreamRevision) -> Result<Self, StreamHistoryError> {
        if events.is_empty() {
            return Err(StreamHistoryError::Empty);
        }

        let event_count =
            u64::try_from(events.len()).map_err(|_| StreamHistoryError::TooManyEvents)?;
        let expected = StreamRevision::new(event_count - 1);
        if revision != expected {
            return Err(StreamHistoryError::RevisionMismatch {
                expected,
                actual: revision,
            });
        }

        Ok(Self { events, revision })
    }

    /// **English:** Borrows all events in construction order.
    ///
    /// **日本語:** 構築時の順序ですべてのイベントを借用します。
    pub fn events(&self) -> &[E] {
        &self.events
    }

    /// **English:** Returns the final event's revision.
    ///
    /// **日本語:** 最後のイベントのリビジョンを返します。
    pub fn revision(&self) -> StreamRevision {
        self.revision
    }

    /// **English:** Consumes the history and returns its events without cloning.
    ///
    /// **日本語:** 履歴を消費し、複製せずにイベントを返します。
    pub fn into_events(self) -> Vec<E> {
        self.events
    }

    /// **English:** Borrows events strictly after `revision`; the final revision yields an empty slice.
    /// Returns `None` for a future revision or if the next index cannot fit in `usize`.
    ///
    /// **日本語:** `revision` より後のイベントを借用します。最終リビジョンなら空スライスです。
    /// 未来のリビジョンや、次の添字が `usize` に収まらない場合は `None` を返します。
    pub fn events_after(&self, revision: StreamRevision) -> Option<&[E]> {
        if revision > self.revision {
            return None;
        }

        let start = usize::try_from(revision.value()).ok()?.checked_add(1)?;
        Some(&self.events[start..])
    }
}

/// **English:** Reason that event data cannot form a complete [`StreamHistory`].
///
/// **日本語:** イベントデータから完全な [`StreamHistory`] を構築できない理由です。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamHistoryError {
    /// **English:** The event vector is empty.
    ///
    /// **日本語:** イベントのベクターが空です。
    Empty,
    /// **English:** The event count exceeds the range of `u64`.
    ///
    /// **日本語:** イベント数が `u64` の範囲を超えています。
    TooManyEvents,
    /// **English:** The final revision differs from the event count minus one.
    ///
    /// **日本語:** 最終リビジョンがイベント数から 1 を引いた値と異なります。
    RevisionMismatch {
        /// **English:** Final revision calculated from the event count.
        ///
        /// **日本語:** イベント数から算出した最終リビジョンです。
        expected: StreamRevision,
        /// **English:** Final revision supplied by the caller.
        ///
        /// **日本語:** 呼び出し側が指定した最終リビジョンです。
        actual: StreamRevision,
    },
}

/// **English:** Stream lookup result, separating absence from complete event history.
///
/// **日本語:** ストリームの検索結果です。不在と完全なイベント履歴を区別します。
#[derive(Debug, Eq, PartialEq)]
pub enum LoadResult<E> {
    /// **English:** The identifier has no stored stream.
    ///
    /// **日本語:** 識別子に対応する保存済みストリームがありません。
    NoStream(NoStream),
    /// **English:** Complete, non-empty history of the stored stream.
    ///
    /// **日本語:** 保存済みストリームの空でない完全な履歴です。
    Found(StreamHistory<E>),
}

/// **English:** Stream position reported after a successful append.
///
/// **日本語:** 追加成功後に報告されるストリーム位置です。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppendResult {
    /// **English:** Revision of the final event after appending the batch.
    ///
    /// **日本語:** バッチ追加後の最後のイベントのリビジョンです。
    pub resulting_revision: StreamRevision,
}

/// **English:** Requested and observed stream states that prevented an append.
///
/// **日本語:** 追加を妨げた、要求されたストリーム状態と観測された状態です。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RevisionConflict {
    /// **English:** Stream state requested by the caller.
    ///
    /// **日本語:** 呼び出し側が要求したストリーム状態です。
    pub expected: ExpectedRevision,
    /// **English:** Stream state observed by the adapter.
    ///
    /// **日本語:** アダプターが観測したストリーム状態です。
    pub current: CurrentRevision,
}

/// **English:** Append failure; `E` carries an adapter-specific storage error.
///
/// **日本語:** 追加の失敗です。`E` はアダプター固有のストレージエラーです。
#[derive(Debug, Eq, PartialEq)]
pub enum AppendError<E> {
    /// **English:** No events were supplied.
    ///
    /// **日本語:** イベントが指定されていません。
    EmptyBatch,
    /// **English:** Expected and observed stream states differ.
    ///
    /// **日本語:** 期待したストリーム状態と観測された状態が異なります。
    Conflict(RevisionConflict),
    /// **English:** Storage adapter failure.
    ///
    /// **日本語:** ストレージアダプターの障害です。
    Adapter(E),
    /// **English:** The batch would exceed the revision range.
    ///
    /// **日本語:** バッチを追加するとリビジョンの範囲を超えます。
    RevisionExhausted {
        /// **English:** Stream revision before the attempted append.
        ///
        /// **日本語:** 追加を試みる前のストリームリビジョンです。
        current: StreamRevision,
    },
}

/// **English:** Synchronous persistence boundary for aggregate event streams.
/// Implementations define atomicity, durability, concurrent-writer coordination, and panic
/// conditions; this trait supplies no storage or locking implementation.
///
/// **日本語:** 集約イベントストリームの同期永続化境界です。
/// 原子性、永続性、並行書き込みの調整、パニック条件は実装が定めます。
/// このトレイト自体はストレージやロックを実装しません。
pub trait EventStore {
    /// **English:** Aggregate defining the identifier and event types to store.
    ///
    /// **日本語:** 保存する識別子とイベントの型を定める集約です。
    type Aggregate: AggregateRoot;
    /// **English:** Adapter-specific storage failure.
    ///
    /// **日本語:** アダプター固有のストレージ障害です。
    type Error;

    /// **English:** Loads the complete history for `id`, or reports an absent stream.
    /// Storage failures are returned as `Self::Error`, not as stream absence.
    ///
    /// **日本語:** `id` の完全な履歴を読み込むか、ストリームの不在を報告します。
    /// ストレージ障害は不在として扱わず、`Self::Error` として返します。
    fn load(
        &self,
        id: &<Self::Aggregate as Entity>::Id,
    ) -> Result<LoadResult<<Self::Aggregate as AggregateRoot>::Event>, Self::Error>;

    /// **English:** Consumes a batch to append in vector order under the `expected` stream state.
    /// Returns the final revision or an [`AppendError`] for an empty batch, conflict, revision
    /// exhaustion, or adapter failure. The batch is not returned on failure; consult the adapter
    /// contract for partial-write and retry behavior.
    ///
    /// **日本語:** バッチを消費し、`expected` のストリーム状態を条件にベクター順で追加します。
    /// 最終リビジョン、または空バッチ・競合・リビジョン枯渇・アダプター障害を示す
    /// [`AppendError`] を返します。失敗時にバッチは返されません。部分書き込みや再試行の
    /// 挙動はアダプターの契約を確認してください。
    fn append(
        &mut self,
        id: &<Self::Aggregate as Entity>::Id,
        expected: ExpectedRevision,
        events: Vec<<Self::Aggregate as AggregateRoot>::Event>,
    ) -> Result<AppendResult, AppendError<Self::Error>>;
}
