# lab3实验报告

## 功能实现

- `os/src/mm/memory_set.rs`：新增 `has_any_mapped`、`is_mapped_range`、`unmap_range`，用于检查虚拟地址区间映射与解除映射。
- `os/src/syscall/process.rs`：
  - 重写 `sys_get_time`，使用 `translated_byte_buffer` 处理跨页写入 `TimeVal`。
  - 实现 `sys_mmap` / `sys_munmap`，支持按页匿名映射与解除映射。
  - 实现 `sys_spawn`，通过 `TaskControlBlock::spawn` 加载 ELF 创建子进程，不复制父进程地址空间。
  - 实现 `sys_set_priority`，校验优先级下限为 2 并更新当前任务优先级。
- `os/src/task/task.rs`：
  - 在 `TaskControlBlockInner` 中新增 `priority`、`stride` 字段。
  - 在 `new`、`fork`、`spawn` 中初始化 `priority = 16`、`stride = 0`。
  - 新增 `spawn` 方法：为子进程新建 `MemorySet`、分配 pid 与内核栈，设置父进程弱引用，初始化 trap context。
- `os/src/task/manager.rs`：修改 `fetch` 为 stride 调度：每次选取 `stride` 最小的就绪任务，调度前 `stride += BIG_STRIDE / priority`。
- `os/src/config.rs`：新增常量 `BIG_STRIDE`。

## 测试结果

运行 `make -C ci-user test CHAPTER=5`，15/15 测例通过。

## 问答作业

### stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

#### 实际情况是轮到 p1 执行吗？为什么？

不是。

p2 跑完一个时间片后 `stride = 250 + 10 = 260`，在 8bit 无符号下溢出为 4。于是 `p2.stride = 4 < p1.stride = 255`，比较器会接着调度 p2，违反预期。

#### 我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明，在不考虑溢出的情况下，在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。为什么？尝试简单说明（不要求严格证明）。

`priority >= 2` 时 `pass = BigStride / priority <= BigStride / 2`。

每次调度只给被选中的进程加一次 pass，所以任意时刻最大 stride 与最小 stride 的差距不会超过一个 pass，即不超过 `BigStride / 2`。

#### 已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 `partial_cmp` 函数，假设两个 Stride 永远不会相等。

用无符号减法算相对距离，差值小于 `BigStride / 2` 说明 self 更小。

```rust
fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    let diff = self.0.wrapping_sub(other.0);
    if diff < BIG_STRIDE / 2 {
        Some(Ordering::Less)
    } else {
        Some(Ordering::Greater)
    }
}
```

## 荣誉准则

在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
- https://learningos.cn/rCore-Tutorial-Guide/chapter5/4exercise.html

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
