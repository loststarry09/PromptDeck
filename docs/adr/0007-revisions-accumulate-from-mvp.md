# Version 数据从 MVP 起累积，历史 UI 后置

自动保存 flush 时，内容哈希变化且距上一修订 ≥10s 才写入一条 Version；MVP 不提供历史 UI，v0.3 直接读取既有数据加界面。

**Considered Options**：v0.3 才建版本表——实现更省，但会丢失 MVP 到 v0.3 之间的全部编辑历史，且届时仍需迁移。

**Consequences**：数据库存在少量用户不可见的写入；10s 合并窗口定义了 v0.3 历史的最小粒度。策略与测试见 `docs/data-model.md`。
