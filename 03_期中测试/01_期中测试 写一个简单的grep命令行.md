# 期中测试: 写一个简单的grep命令行

今天我们要做的一个小工具是rgrep, 它是一个类似grep的工具, 如果你是一个*nix用户, 那大概率使用过grep或者ag这样的文本查找工具

grep用于查找文件里符合条件的字符串, 如果发现某个文件的内容符合所指定的字符串, grep命令会把含有字符串的哪一行显示出来; 如果指定任何文件名称, 或者所给与的文件名-, grep命令会在标准输入设备读取数据

我们的rgrep要稍微简单一点, 它可以支持一下三种场景:

首先最简单的, 给定义一个字符串以及一个文件, 打印出文件中所有包含改行中的字符串的行:

```bash
$ rgrep Hello a.txt
55: Hello world. This is an exmaple text
```

然后放宽限制, 允许用户提供一个正则表达式, 来查找文件中欧冠所有包含该字符串的行:

```bash
$ rgrep Hel[^\\s]+ a.txt
55: Hello world. This is an exmaple text
89: Help me! I need assistant!
```

如果这个也可以实现, 那进一步放宽限制, 允许用户提供一个正则表达式, 来查找满足文件通配符的所有文件(你可以使用globset或者glob来出来通配符), 比如:

```bash
$ rgrep Hel[^\\s]+ a*.txt
a.txt
55:1 Hello world. This is an exmaple text
89:1 Help me! I need assistant!
5:6 Use `Help` to get help.
abc.txt:
100:1 Hello Tyr!
```

其中, 冒号前面的数字是行号, 后面的数字是字符在这一行的位置

给你一点小提示:

- 对于命令行的部分, 你可以使用clap或者structopt, 也可以使用`env.args`
- 对于正则表达式的支持, 可以使用regex
- 置于文件的读取, 可以使用`std::fs`或者`tokio::fs`, 你可以顺序对所有满足通配符的文件进行处理, 也可以用rayon或者tokio来并行处理
- 对于输出的结果, 最好能够匹配文件用不同的颜色展示

如果你有余力, 可以看看grep的文档, 尝试实现更多的功能

