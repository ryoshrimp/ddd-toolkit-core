# ddd-toolkit-core

[English](README.md) | 日本語

[![crates.io](https://img.shields.io/crates/v/ddd-toolkit-core.svg)](https://crates.io/crates/ddd-toolkit-core)
[![docs.rs](https://img.shields.io/docsrs/ddd-toolkit-core)](https://docs.rs/ddd-toolkit-core)
[![CI](https://github.com/ryoshrimp/ddd-toolkit-core/actions/workflows/ci.yml/badge.svg)](https://github.com/ryoshrimp/ddd-toolkit-core/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/ryoshrimp/ddd-toolkit-core/graph/badge.svg)](https://codecov.io/gh/ryoshrimp/ddd-toolkit-core)
[![MSRV](https://img.shields.io/crates/msrv/ddd-toolkit-core)](Cargo.toml)
[![license](https://img.shields.io/crates/l/ddd-toolkit-core.svg)](#ライセンス)

Rust でドメイン駆動設計を実践するためのコアトレイトとヘルパーです。
エンティティ、値オブジェクト、集約、永続化、CQRS、イベント駆動のワークフローを扱います。

永続化フレームワークではなく、型付きの境界を定義するクレートです。
ドメイン型とアダプターの実装、エラー型の選択、ストレージ、トランザクション、配信方針、
必要に応じた非同期ランタイムは利用側が用意します。マクロへの依存はありません。
公開される構成要素はすべて `ddd_toolkit_core` のルートから再エクスポートされます。

## インストール

```sh
cargo add ddd-toolkit-core
```

**Rust 1.85 以降**（edition 2024）が必要です。デフォルト feature はありません。
唯一のオプション feature である `serde` は、生成された実装から serde を利用するための
内部公開を有効にします。ドメイン型や `Secret` が自動的にシリアライズ可能になるわけではありません。
秘密値の消去に `zeroize` を使用します。非同期ランタイムへの依存はありません。

## 構成要素

| 分野 | 公開 API |
| --- | --- |
| ドメインモデル | `Entity`, `ValueObject`, `AggregateRoot`, `DomainEvent` |
| 永続化 | `Repository`, `AsyncRepository` |
| イベント発行 | `DomainEventPublisher`, `AsyncDomainEventPublisher`, `dispatch_events`, `dispatch_events_async` |
| イベントの記述 | `DomainEventMetadata`, `DomainEventConversion`, `DeliveryMetadata`, `DomainEventDeliveryMetadata` |
| CQRS | `CommandHandler`, `AggregateCreator`, `QueryHandler`、ライフサイクルのコンテキスト／コールバック、およびそれぞれの `Async` 版 |
| イベントソーシング | `EventStore`, `AsyncEventStore`、リビジョン／履歴の型、`AggregateReplay`, `replay_aggregate` |
| スナップショット | `AggregateSnapshot`, `SnapshotStore`, `AsyncSnapshotStore`, `replay_with_optional_snapshot` |
| イベント消費 | `EventConsumer`, `Projection`、およびそれぞれの `Async` 版 |
| 信頼性を考慮したプロジェクション | `ProjectionCheckpoint`, `ProjectionCheckpointStore`, `IdempotentProjection`, `ProjectionApplication`、および非同期ポート |
| トランザクショナル Outbox | `TransactionalOutbox`, `AsyncTransactionalOutbox`, `OutboxClaim`, `OutboxMessageId`、およびライフサイクルコンテキスト |
| ドメインルール | `Specification`, `And`, `Or`, `Not`, `DomainService`, `Policy`, `DomainFactory` |
| ワークフローとトランザクション | `ProcessManager`, `Transition`, `UnitOfWork` |
| スキーマの進化 | `EventIdentity`, `VersionedEvent`, `EventUpcaster`, `UpcasterChain` |
| 秘密値 | `Secret`, `SecretMetadata` |

非同期ポートはボックス化された `Send` な Future を返します。実装と呼び出し側は、
各 API のライフタイムとスレッド安全性の制約を満たす必要があります。
このクレートが executor を選択することはありません。

## 例：集約、リポジトリ、イベント発行

識別子はアプリケーションで定義します。`Entity::Id` に必須のマーカートレイトはありません。
集約は `record_event` でイベントを記録し、`pull_events` で保留イベントの所有権を渡します。
永続化とイベント発行は別々の操作です。

この例では、インメモリのリポジトリとパブリッシャーを独自に実装しています。
どちらも組み込みアダプターではありません。

```rust
use ddd_toolkit_core::{
    AggregateRoot, DomainEvent, DomainEventPublisher, Entity, Repository,
    dispatch_events,
};
use std::{collections::HashMap, convert::Infallible};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OrderId(String);

#[derive(Debug, Clone)]
struct OrderPlaced;

impl DomainEvent for OrderPlaced {}

#[derive(Clone)]
struct Order {
    id: OrderId,
    events: Vec<OrderPlaced>,
}

impl Entity for Order {
    type Id = OrderId;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

impl AggregateRoot for Order {
    type Event = OrderPlaced;

    fn record_event(&mut self, event: Self::Event) {
        self.events.push(event);
    }

    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

#[derive(Default)]
struct OrderRepository(HashMap<OrderId, Order>);

impl Repository for OrderRepository {
    type Aggregate = Order;
    type Error = Infallible;

    fn find_by_id(&self, id: &OrderId) -> Result<Option<Order>, Self::Error> {
        Ok(self.0.get(id).cloned())
    }

    fn save(&mut self, aggregate: &Order) -> Result<(), Self::Error> {
        // 配信待ちキューではなく、ドメインの状態を保存します。
        self.0.insert(aggregate.id.clone(), Order {
            id: aggregate.id.clone(),
            events: Vec::new(),
        });
        Ok(())
    }

    fn delete(&mut self, id: &OrderId) -> Result<(), Self::Error> {
        self.0.remove(id);
        Ok(())
    }

    fn exists(&self, id: &OrderId) -> Result<bool, Self::Error> {
        Ok(self.0.contains_key(id))
    }
}

#[derive(Default)]
struct Publisher(Vec<OrderPlaced>);

impl DomainEventPublisher for Publisher {
    type Event = OrderPlaced;
    type Error = Infallible;

    fn publish(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
        self.0.push(event.clone());
        Ok(())
    }
}

fn main() -> Result<(), Infallible> {
    let mut repository = OrderRepository::default();
    let mut publisher = Publisher::default();
    let mut order = Order {
        id: OrderId("order-1".into()),
        events: Vec::new(),
    };
    order.record_event(OrderPlaced);

    repository.save(&order)?; // 集約のイベントは取り出しません。
    dispatch_events(&mut order, &mut publisher)?;

    assert!(order.pull_events().is_empty());
    assert_eq!(publisher.0.len(), 1);
    let loaded = repository.find_by_id(order.id())?.expect("saved order");
    assert_eq!(loaded.id(), order.id());
    Ok(())
}
```

`dispatch_events` は `pull_events` が返した順序で発行します。
発行に失敗すると、パブリッシャーがすでに受理した分も含めて、取り出したイベントを**すべて**復元し、
パブリッシャーのエラーをそのまま返します。このため、再試行で重複配信が発生する可能性があります。
非同期版の `dispatch_events_async` は、処理中の Future が破棄された場合にも取り出したイベントを復元します。
どちらのヘルパーも、永続化と発行をアトミックにはしません。

## 値オブジェクトと秘密値

`ValueObject` は内包値を公開します。検証や等価性の意味は、ドメイン型の実装が定めます。

```rust
use ddd_toolkit_core::ValueObject;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CustomerName(String);

impl ValueObject for CustomerName {
    type Inner = String;

    fn as_inner(&self) -> &Self::Inner {
        &self.0
    }

    fn into_inner(self) -> Self::Inner {
        self.0
    }
}

let name = CustomerName("Alice".into());
assert_eq!(name.as_inner(), "Alice");
assert_eq!(name.into_inner(), "Alice");
```

`Secret<T>` は `T: zeroize::Zeroize` を要求し、`Debug` と `Display` の出力を秘匿化して、
破棄時に内包値をゼロ化します。平文へのアクセスは明示的に行います。

```rust
use ddd_toolkit_core::Secret;

let token = Secret::new(String::from("example-token"));
assert_eq!(format!("{token}"), "[REDACTED]");
assert!(token.expose_secret(|value| value.starts_with("example-")));
```

`as_inner` でも平文にアクセスできます。`into_inner` を呼ぶと、以後のゼロ化の責任は呼び出し側に移ります。
`SecretMetadata` はサイズがゼロのマーカーにすぎず、値を保護しません。
`Secret` は serde によるシリアライズや暗黙の参照外しを実装しません。

## ワークフローと信頼性の責任範囲

- **CQRS：** 型付きのリポジトリ、パブリッシャー、ライフサイクルコールバックを使って、
  コマンドハンドラーや集約作成器を実装します。ライフサイクルコンテキストは保存／発行の試行を追跡し、
  未実行や未成功の操作を検出しますが、トランザクションは調整しません。クエリハンドラーは共有参照を使います。
- **イベントソーシング：** イベントストアは検証済みの完全な `StreamHistory` を読み込み、
  `ExpectedRevision` を条件に追記します。再生フックでドメイン状態を復元し、
  スナップショットストアはスナップショット状態を別途保存します。並行処理と永続化の保証はアダプターが提供します。
- **Outbox：** アダプターがアトミックな保存・登録、取得、確認応答を実装します。
  ライフサイクルコンテキストは一回の登録試行を追跡し、保留イベントを消費して、失敗時にも復元しません。
  非同期コンテキストはポーリング前に試行を消費してイベントを取り出し、Future の破棄でも復元しません。
  コンテキスト自体はトランザクションを実装しません。
- **プロジェクション：** チェックポイントストアと冪等なプロジェクションが、進捗と適用済み／重複の結果を公開します。
  重複検出と、読み取りモデル変更・チェックポイント永続化の連携は実装側の責任です。
- **その他のワークフロー：** 仕様は失敗し得る述語を短絡評価で結合します。
  プロセスマネージャーは状態と副作用の記述を返し、呼び出し側が保存・実行します。
  Unit of Work の実装がコミットとロールバックの保証を定めます。
  Upcaster のチェーンは追加順に変換段階を適用し、スキーマバージョンを自動選択しません。

非同期ポートの Future を破棄しても、一般にロールバックや副作用の不在は保証されません。
キャンセル、再試行、配信保証はアダプターとアプリケーションで定義してください。

API 契約は `cargo doc --all-features --open` を実行して確認できます。
具体的な実装、非同期の利用例、コンパイル時の境界チェックは
[結合テスト](tests)を参照してください。

## 変更履歴

リリース履歴は [CHANGELOG.md](CHANGELOG.md) を参照してください。

## ライセンス

[Apache License, Version 2.0](LICENSE-APACHE) または [MIT license](LICENSE-MIT) の
いずれかを選択して利用できます。
