## Report

### 实现功能总结

原先的rCore尽管在`exec`中实现了通过参数`args: Vec<String>`传入argc和argv，但是在下面的处理过程中写入内存的方式和elf的文件规范是刚好相反的，elf文件规范是自下而上分别为argc，argv的地址，argv的数据；而rCore则是自上而下的，因此通过重新处理args数据在用户栈中的排布，便可以实现带argc和argv的程序运行
而原先的rCode尽管进行了args的处理，但实际上所有测例都不会有args的传入，因此即便不处理args都不会影响原先的测例的运行，而我在实现的过程中也直接选择屏蔽原先`exec`中的args，在方法内部重新写了一个新的args

### 问答题
1. elf 文件和 bin 文件有什么区别？
    > Gcc 编译出来的是 ELF 文件。 通常 gcc –o test test.c, 生成的 test 文件就是 ELF 格式的，在 linuxshell 下输入./test 就可以执行。 Bin 文件是经过压缩的可执行文件，去掉 ELF 格式的东西。 是直接的内存映像的表示

Linux 的 file 命令可以检查文件的类型，尝试执行以下命令，描述看到的现象，然后尝试解释输出
```
# file ch6_file0.elf
ch6_file0.elf: ELF 64-bit LSB executable, UCB RISC-V, RVC, double-float ABI, version 1 (SYSV), statically linked, stripped
# file ch6_file0.bin
ch6_file0.bin: data
# riscv64-linux-musl-objdump -ld ch6_file0.bin > debug.S
riscv64-linux-musl-objdump: ch6_file0.bin: file format not recognized
```
运行`file ch6_file0.elf`看到elf文件包含了很多信息，比如段表，符号表，动态链接表等等，而bin文件只包含了二进制代码。

## 我的说明

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与以下各位就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    > 无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    > [rCore-Tutorial-Guide-2023A](https://learningos.cn/rCore-Tutorial-Guide-2023A)  
    > [rCore-Tutorial-V3](https://rcore-os.cn/rCore-Tutorial-Book-v3)  
    > [实验指导书](https://scpointer.github.io/rcore2oscomp/)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计
