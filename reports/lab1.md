# lab1实验报告
## 功能实现
在任务控制块TCB内新增syscall_counts数组，每个进程独立记录各系统调用次数；在syscall入口处调用任务管理器方法，对当前进程对应系统调用id的计数自增，由任务管理器维护计数。任务管理器实现increment_syscall、get_syscall_count方法，并在task模块对外导出这两个函数，供syscall模块调用。

## 一. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。请同学们可以自行测试这些内容（运行 https://github.com/LearningOS/rCore-Tutorial-Test/tree/master/src/bin ），描述程序出错行为，同时注意注明你使用的 sbi 及其版本。
SBI版本：RustSBI-QEMU Version 0.1.0
测试仓库：rCore-Tutorial-Test

### 出错行为：
1. `illegal_inst` 应用：用户态尝试执行S态特权指令、访问S态CSR（如 sstatus、sepc 等），硬件抛出**非法指令异常 IllegalInstruction**，内核捕获异常后直接杀死该应用，对应日志：
`[kernel] IllegalInstruction in application, kernel killed it.`（出现两次，对应两组非法指令测试）

2. `bad_addr` 应用：用户态访问虚拟地址 0x0，触发页故障 PageFault，内核捕获异常并杀死应用，对应日志：
`[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.`


## 二. 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:
### Q1：刚进入 __restore 时，sp 代表了什么值？请指出 __restore 的两种使用情景。
sp 指向内核栈上 `TrapContext` 的起始地址

1. 用户态异常/中断处理完毕，从内核态切换回用户态
2. 切换到新的app前设置相应的app初始状态

### Q2：L43-L48 这几行汇编特殊处理了哪些 CSR？这些寄存器的值对于进入用户态有何意义？
- `sstatus`：保存特权标记 `SPP`，`sret` 依靠它判断返回U/S态；
- `sepc`：保存返回地址，`sret` 跳转至此；
- `sscratch`：暂存用户栈指针，为后续恢复用户sp做准备。

### Q3：L50-L56 为何跳过了 x2 和 x4？
- x2（sp）：此时仍在内核栈运行，提前恢复会破坏内核栈；延后通过 `csrrw` 交换恢复；
- x4（tp）：rCore约定用户程序不使用tp，无需保存恢复。

### Q4：执行 `csrrw sp, sscratch, sp`（L60）之后，sp 和 sscratch 分别是什么意义？
- sp：用户栈指针；
- sscratch：保存原来的内核栈指针，供下次trap交换使用。

### Q5：__restore 中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
切换指令是 `sret`。
`sret` 读取 `sstatus.SPP`，此处SPP=0，硬件将特权级切为U态，跳转到 `sepc`。

### Q6：`__alltraps` 开头 `csrrw sp, sscratch, sp` 执行后，sp 和 sscratch 含义？
- sp：内核栈指针；
- sscratch：暂存用户栈指针。

### Q7：从 U 态进入 S 态是哪一条指令发生的？
`ch2，ch3`app通过`syscall`主动陷入内核，使用的是`ecall`指令



## 荣誉准则
在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
- https://learningos.cn/rCore-Tutorial-Guide/chapter3/5exercise.html

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
