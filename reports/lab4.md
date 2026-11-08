# lab4实验报告
## 功能实现
在 easy-fs 中，DiskInode 新增 nlink 字段（initialize 时置 1）用于记录硬链接数；EasyFileSystem 新增 get_disk_inode_id（由 block_id 与块内偏移反推 inode 编号）与 dealloc_inode（回收 inode 位图中对应槽位）；Inode 新增 link、unlink、stat 三个方法：link 在当前目录追加新目录项并对目标 inode 的 nlink 自增，unlink 通过重写目录 dirent 删除指定项，对目标 inode 的 nlink 自减，nlink 减到 0 时回收其数据块与 inode 本身，stat 借助 get_disk_inode_id 返回 (dev, ino, mode, nlink)。

在 os 层，fs/inode.rs 与 fs/mod.rs 为 OSInode 补充 get_nlink/get_inode_id/get_mode/get_stat_info，File trait 增加 stat_info 接口，Stat 结构补充 new 构造函数；syscall/fs.rs 实现 sys_fstat（从 fd 表取出 stat_info 写回用户 Stat）、sys_linkat、sys_unlinkat（翻译用户字符串后直接调用 ROOT_INODE 的 link/unlink）。此外，从 ch5 迁移补全了 sys_get_time（按页翻译 TimeVal，处理跨页写入）、sys_mmap/sys_munmap（memory_set.rs 新增 has_any_mapped、unmap_range 等，munmap 时按区域重叠情况拆分/收缩映射）、sys_spawn（TaskControlBlock 新增 spawn 方法，直接为子进程建立全新地址空间并注册父子关系）、sys_set_priority，其中 task.rs 为 TCB 新增 priority/stride 字段，manager.rs 的 fetch 改为 stride 调度（选 stride 最小者，运行后加上 BIG_STRIDE/priority），config.rs 中新增 BIG_STRIDE 常量。

## 问答作业
### 1. 在我们的easy-fs中，root inode起着什么作用？如果root inode中的内容损坏了，会发生什么？

root inode 是整个文件系统的"根目录"，它是用户找到其他所有文件的唯一入口。EasyFileSystem::root_inode 返回根目录的 Inode，之后所有查找（find/create/link/unlink）都是从根目录的 dirent 逐级向下找到子目录或文件的。目录树里任何一个文件，都得从 root inode 的 dirents 之一出发，沿着 dirent 记录的 inode id 继续往下找。

如果 root inode 内容损坏了，会发生什么？root inode 的 size 或者 dirent 数据坏了，会导致整个文件系统的根目录无法正常解析，进而导致用户程序无法访问任何文件。

### ch7 问答题

#### 1. 举出使用 pipe 的一个实际应用的例子。

最常见的就是统计文件行数：`cat foo.txt | wc -l`。cat 把 foo.txt 的内容写到自己的 stdout，内核把这个 stdout 通过管道接到了 wc 的 stdin 上，于是 wc 读到的就是 cat 的输出，数完行数后打印。shell 里凡是写成 "cmd1 | cmd2" 的命令行，都是把 cmd1 的输出接到 cmd2 的输入，比如 grep error | sort，出水口给入水口，中间不需要落盘，两个进程各自按普通读写文件的方式用 0/1 号 fd 就完成了协作。

举例日常用的：
```sh
cat foo.txt | grep "hello" | wc -l
```

#### 2. 如果需要在多个进程间互相通信，则需要为每一对进程建立一个管道，非常繁琐，请设计一个更易用的多进程通信机制。

N 个进程两两建管道要 O(N²) 个管道，管理起来很痛苦。可以设计一个内核提供的"信箱/消息队列"机制：

- 内核里维护一个全局的消息中心（mailbox），每个进程可以有若干个命名信箱（message queue），给它起个 id 或名字，别的进程凭 id 申请关联。
- 任意进程可以 send(msg) 往信箱里投消息，也可以 recv() 从信箱取消息；信箱对不特定进程开放，发消息和收消息不需要预先两两配对，N 个进程只要各自跟一个信箱交互即可，沟通开销从 O(N²) 降为 O(N)。
- 消息带上发送者的 pid 和优先级，接收方可按优先级/发布时间处理；内核要处理好同步：信箱空时 recv 阻塞或带超时，满时 send 阻塞；
- 用环形缓冲或者链表存消息，并配互斥锁/条件变量（或信号量）保证并发安全。

简单说就是把"点对点的管道"升级成"广播式的邮局"，所有要通信的进程都持有一个地址是共有的信箱，发件和收件都走信箱，就不再需要每对进程单独连线了。

## 荣誉准则
### 1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

### 2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

- https://learningos.cn/rCore-Tutorial-Guide/chapter6/4exercise.html

### 3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

### 4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
