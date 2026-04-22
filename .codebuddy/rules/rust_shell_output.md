---
description: 
alwaysApply: true
enabled: true
updatedAt: 2026-04-16T12:36:11.849Z
provider: 
---

**⚠️ Rust 编译耗时注意事项**

由于 Rust 编译时间较长，在执行 shell 命令（如 `cargo check`、`cargo clippy`、`cargo test`）时，
可能因超时而无法直接获取到 shell 输出。遇到此情况时，应采用**后台执行 + 标记文件 + 轮询等待**的方式。

# 执行策略：后台运行 + 完成标记文件

```bash
# Step 1: 后台执行命令，将输出重定向到临时文件，完成后写入退出码到标记文件
(cargo make check 2>&1 | tee /tmp/cargo-check-output.txt; echo $? > /tmp/cargo-check-done) &

# Step 2: 轮询等待标记文件出现（表示命令已执行完毕）
# 每隔几秒检查一次 /tmp/cargo-check-done 是否存在
while [ ! -f /tmp/cargo-check-done ]; do sleep 10; done

# Step 3: 读取退出码，判断是否成功
cat /tmp/cargo-check-done    # 0 表示成功，非 0 表示失败

# Step 4: 读取完整输出
cat /tmp/cargo-check-output.txt
```
**当上一个轮询命令没有结束时， 不能进行下一次的轮询**

# 三项门禁的完整执行示例

**必须严格串行执行**，因为 Cargo 使用文件锁（`cargo.lock`），多个 cargo 命令不能并行运行，否则会因锁冲突而失败。

```bash
# ---- 1. cargo check ----
rm -f /tmp/cargo-check-done /tmp/cargo-check-output.txt
(cargo check 2>&1 | tee /tmp/cargo-check-output.txt; echo $? > /tmp/cargo-check-done) &
# 轮询等待完成
while [ ! -f /tmp/cargo-check-done ]; do sleep 10; done
# 检查结果：读取 /tmp/cargo-check-done（应为 0）和 /tmp/cargo-check-output.txt

# ---- 2. cargo clippy（仅在 check 通过后执行）----
rm -f /tmp/cargo-clippy-done /tmp/cargo-clippy-output.txt
(cargo clippy -- -D warnings 2>&1 | tee /tmp/cargo-clippy-output.txt; echo $? > /tmp/cargo-clippy-done) &
while [ ! -f /tmp/cargo-clippy-done ]; do sleep 10; done
# 检查结果

# ---- 3. cargo test（仅在 clippy 通过后执行）----
rm -f /tmp/cargo-test-done /tmp/cargo-test-output.txt
(cargo test 2>&1 | tee /tmp/cargo-test-output.txt; echo $? > /tmp/cargo-test-done) &
while [ ! -f /tmp/cargo-test-done ]; do sleep 10; done
# 检查结果
```

# ⚠️ Cargo 文件锁注意事项

- Cargo 在编译时会获取 `target/` 目录下的文件锁，**同一时间只能运行一个 cargo 命令**
- 如果前一个 cargo 命令尚未结束就启动下一个，后者会阻塞等待锁释放，或直接报错
- 因此三项门禁检查**必须严格按顺序串行执行**：`cargo make check` → `cargo make clippy` → `cargo make test`
- 修复cargo make clippy 的问题， 不使用添加 allow 属性来方式来处理
- 每一步必须确认**标记文件已生成**（即命令已完成）后，才能启动下一步
- 执行前务必 `rm -f` 清理上一轮的临时文件和标记文件，避免误读旧结果 
- 一般的Cargo 编译也使用**后台执行 + 标记文件 + 轮询等待**的方式


# 文件的大小需要注意

- 文件要有合理的划分， 不要让文件过大, 要符合rust的最佳实践
- 如果文件中测试数据过大，统一采用 #[cfg(test)] mod tests; 拆分策略

# 前端组件的书写注意
- 每个文件中只能有一个组件，多个组件不能在一个文件中
- html的代码块中标签的缩进使用2个空格