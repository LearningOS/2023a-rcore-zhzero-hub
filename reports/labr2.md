## Report

### 实现功能总结

为了实现调用了标准库的hellostd，除了实验指导书的步骤外，根据报错提示，还需要实现以下四个系统调用：
1. `SYSCALL_SET_TID_ADDRESS`：根据指导书，只需要返回`sys_getpid`
2. `SYSCALL_IOCTL`：用于操作特殊文件的底层设备参数，与实验无关，直接返回0
3. `SYSCALL_WRITEV`：本质上是数组形式的`SYSCALL_WRITE`，需要将数组中的每个数组都写入对应地址，查询SYSCALL手册可知`IoVec`的结构体是如何定义的，仿照手册中c语言的形式转义成rust即可，本质上就是SYSCALL_WRITE加了个for循环
4. `SYSCALL_EXIT_GROUP`：与本实验无关，直接返回0

## 问答题
1. 查询标志位定义。
标准的 waitpid 调用的结构是 pid_t waitpid(pid_t pid, int *_Nullable wstatus, int options);。其中的 options 参数分别有哪些可能（只要列出不需要解释），用 int 的 32 个 bit 如何表示？
waitpid 函数的 options 参数是一个位掩码，可以通过按位或（|）来组合多个选项。以下是可能的选项：
WNOHANG     
WUNTRACED   
WCONTINUED  
WNOWAIT     
__WNOTHREAD 
__WALL      
__WCLONE    
这些值可以通过一个32位的int使用按位或运算符进行组合，例如：WNOHANG | WUNTRACED 表示同时设置 WNOHANG 和 WUNTRACED 选项


## 我的说明

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与以下各位就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    > 无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    > [rCore-Tutorial-Guide-2023A](https://learningos.cn/rCore-Tutorial-Guide-2023A)  
    > [rCore-Tutorial-V3](https://rcore-os.cn/rCore-Tutorial-Book-v3)  
    > [实验指导书](https://scpointer.github.io/rcore2oscomp/)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计