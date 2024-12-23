# 异步处理: async/await内部是怎么实现的?

学完上一讲, 我们对Future和async/await的基本概念有一个比较扎实的理解了, 知道在什么情况下该使用Future, 什么情况下该使用Thread, 以及executor和reactor是怎么联动最终让Future得到了一个结果

然而, 我们并不清楚为什么async fn或者async block就能够产生Future, 也并不明白Future是怎么被executor处理的, 今天哦我们就继续深入下去, 看看async/await这两个关键字究竟施了什么魔法, 能让一切如此简单有如此自然的运转起来

提前说明一下, 我们会继续围绕着Future这个简约却又不简单的结构, 来探讨一些原理性的问题, 主要是Context和Pin

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

这堂课的内容即便没有完全搞懂, 也不影响你继续使用async/await

### Waker的调用机制

先来看这个接口的Context是个什么东西

上一节我们简单讲过executor是通过poll方法来让Future继续往下执行, 如果调用Weaker.wake把Future唤醒, 这个Waker是哪里的呢?

其实, 它隐含在Context中:

```rust
pub struct Context<'a> {
    waker: &'a Waker,
    _marker: PhantomData<fn(&'a ()) -> &'a ()>,
}
```

所以, Context就是Waker的一个封装

如果你去看Waker的定义和相关的代码, 会发现它非常的抽象, 内部使用了一个vtable来运行各种各样的waker的行为:

```rust
pub struct RawWakerVTable {
    clone: unsafe fn(*copnst ()) -> RawWakerm
    wake: unsafe fn(*const ()),
    wake_by_ref: unsafe fn(*const ()),
    drop: unsafe fn(*const ()),
}
```

这种手工生成vtable的做法, 我们之前阅读bytes的源码已经见识过了, 它可以最大程度的兼顾效率和灵活性

Rust自身并不提供异步运行时, 它只在标准库中规定了一些基本的接口, 至于怎么实现, 可以由各个运行时自行决定, 所以在标准库中, 你只会看到这些接口的定义, 以及高层接口的实现, 比如Waker下的wake方法, 只是调用了vtable里的wake而已:

```rust
impl Waker {
    /// Wake up the task associated with this `Waker`.
    #[inline]
    pub fn wake(self) {
        // The actual wakeup call is delegated through a virtual function call
        // to the implementation which is defined by the executor.
        let wake = self.waker.vtable.wake;
        let data = self.waker.data;
        // Don't call `drop` -- the waker will be consumed by `wake`.
        crate::mem::forget(self);
        // SAFETY: This is safe because `Waker::from_raw` is the only way
        // to initialize `wake` and `data` requiring the user to acknowledge
        // that the contract of `RawWaker` is upheld.
        unsafe { (wake)(data) };
    }
    ...
}
```

如果你想要顺藤摸瓜找到vtable是这么设置的, 却发现一切线索都悄无声息的中断了, 那是因为, 具体的实现并不在标准库中, 而是在第三方的异步运行时里, 比如tokio

不过, 虽然我们开发的时候使用tokio, 但是阅读和理解代码的时候, 建议看futures库, 比如waker vtable的定义, futures库还有一个简单的executor, 也非常适合进一步通过代码理解executor的原理

## async究竟生成了什么?

我们接下来看Pin, 这是一个奇怪的数据结构, 正常的数据结构都是直接使用self/&self/&mut self, 可视poll却使用了`Pin<&mut self>`, 为什么?

为了讲明白Pin, 我们的往前追踪异步, 看看产生Future的一个async block/fn内部进行生成什么样的代码? 

```rust
async fn write_hello_file_async(name: &str) -> anyhow::Result<()> {
    let mut file = fs::File::create(name).await?;
    file.write_all(b"hello world!").await?;
    Ok(())
}
```

首先它创建一个文件, 然后往这个文件里写入hello world, 这两个函数有两个await, 创建文件的时候会异步创建, 写入文件的时候会异步写入, 最终整个函数对外返回一个Future

我们知道executor处理Future时, 会不断地调用poll方法, 于是上面那句实际上相当于:

```rust
match write_hello_file_async.poll(cx) {
    Poll::Ready(result) => return result,
    Poll::Pending => return Poll::Pending,
}
```

这就是单个await的处理方法, 那更加复杂的, 一个函数中有若干个await, 那该如何处理呢? 以前面write_hello_file_async函数的内部实现为例, 显然我们只有处理完create, 才能处理write_all, 所以应该是类似这样的代码:

```rust
let fut = fs::File::create(name);
match fut.poll(cx) {
    Poll::Ready(Ok(file)) => {
        let fut = file.write_all(b"hello world");
        match fut.poll(cx) {
            Poll::Ready(result) => return result,
            Poll::Pending =< return Poll::Pending,
        }
    },
    Poll::Pending => return Poll;:Pending,
}
```

但是前面说过, async函数返回的是一个Future, 所以还是需要把这样的代码封装在一个Future的实现李, 对外提供出去, 因此我们需要实现一个数据结构, 把内部的状态保存起来, 并未这个数据结构实现Future, 比如:

```rust
enum WriteHelloFile {
    // 初始阶段，用户提供文件名
    Init(String),
    // 等待文件创建，此时需要保存 Future 以便多次调用
    // 这是伪代码，impl Future 不能用在这里
    AwaitingCreate(impl Future<Output = Result<fs::File, std::io::Error>>),
    // 等待文件写入，此时需要保存 Future 以便多次调用
    AwaitingWrite(impl Future<Output = Result<(), std::io::Error>>),
    // Future 处理完毕
    Done,
}
impl WriteHelloFile {
    pub fn new(name: impl Into<String>) -> Self {
        Self::Init(name.into())
    }
}
impl Future for WriteHelloFile {
    type Output = Result<(), std::io::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
    todo!()
}
}
fn write_hello_file_async(name: &str) -> WriteHelloFile {
    WriteHelloFile::new(name)
}
```

这样, 我们就把刚才的write_hello_file_async异步函数, 转换成了一个返回WriteHelloFile Future的函数, 来看看这个Future如何实现:

```rust
impl Future for WriteHelloFile {
    type Output = Result<(), std::io::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>{
        let this = self.get_mut();
        loop {
            match this {
                // 如果状态是 Init，那么就生成 create Future，把状态切换到 AwaitingC
                WriteHelloFile::Init(name) => {
                    let fut = fs::File::create(name);
                    *self = WriteHelloFile::AwaitingCreate(fut);
                }
                // 如果状态是 AwaitingCreate，那么 poll create Future
                // 如果返回 Poll::Ready(Ok(_))，那么创建 write Future
                // 并把状态切换到 Awaiting
                WriteHelloFile::AwaitingCreate(fut) => match fut.poll(cx) {
                    Poll::Ready(Ok(file)) => {
                        let fut = file.write_all(b"hello world!");
                        *self = WriteHelloFile::AwaitingWrite(fut);
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                },
                // 如果状态是 AwaitingWrite，那么 poll write Future
                // 如果返回 Poll::Ready(_)，那么状态切换到 Done，整个 Future 执行成功
                WriteHelloFile::AwaitingWrite(fut) => match fut.poll(cx) {
                    Poll::Ready(result) => {
                        *self = WriteHelloFile::Done;
                        return Poll::Ready(result);
                    }
                    Poll::Pending => return Poll::Pending,
                },
                // 整个 Future 已经执行完毕
                WriteHelloFile::Done => return Poll::Ready(Ok(())),
            }
        }
    }
}
```

这个Future完整实现的内部结构, 其实就是一个状态机的迁移

这段伪代码和之前异步函数是等价的:

```rust
async fn write_hello_file_async(name: &str) -> anyhow::Result<()> {
    let mut file = fs::File::create(name).await?;
    file.write_all(b"hello world!").await?;
    Ok(())
}
```

Rust在编译async fn或者async block, 就会生成类似的状态机实现, 你可以看到看似简单的异步处理, 内部隐藏着一套并不太难理解, 但写起来很生硬很啰嗦的状态机管理代码

搞明白这个问题, 回到Pin, 刚才我们胡搜学状态机代码的过程, 能帮你理解为什么会需要Pin这个问题

## 为什么需要Pin?

在上面实现Future的状态机中, 我们引用了file这样一个局部变量:

```rust
WriteHelloFile::AwaitingCreate(fut) => match fut.poll(cx) {
    Poll::Ready(Ok(file)) => {
        let fut = file.write_all(b"hello world!");
        *self = WriteHelloFile::AwaitingWrite(fut);
    }
    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
    Poll::Pending => return Poll::Pending,
}
```

这个代码是有问题的, file被fut引用, 但file会在这个作用域被丢弃, 所有我们需要把它保存在数据结构中:

```rust
enum WriteHelloFile {
    // 初始阶段，用户提供文件名
    Init(String),
    // 等待文件创建，此时需要保存 Future 以便多次调用
    AwaitingCreate(impl Future<Output = Result<fs::File, std::io::Error>>),
    // 等待文件写入，此时需要保存 Future 以便多次调用
    AwaitingWrite(AwaitingWriteData),
    // Future 处理完毕
    Done,
}
struct AwaitingWriteData {
    fut: impl Future<Output = Result<(), std::io::Error>>,
    file: fs::File,
}
```

可以生成一个AwaitingWriteData数据结构, 把file和fut都放进去, 然后在WriteHelloFile中引用它, 此时在同一个数据结构内部, fut指向了对file的引用, 这样的数据结构, 叫自引用结构(Self-Referential Structure)

自引用结构由一个很大的问题: 一旦它被移动, 原本的指针就会指向旧的地址

![image-20241223170824818](assets/image-20241223170824818.png)

所以需要有某种机制来保证这种情况不会发生, Pin就是为了这个目的而设计的数据结构, 我们可以Pin住一个指向Future的指针, 看文档中Pin的声明:

```rust
pub struct Pin<P> {
    pointer: P,
}
impl<P: Deref> Deref for Pin<P> {
    type Target = P::Target;
    fn deref(&self) -> &P::Target {
        Pin::get_ref(Pin::as_ref(self))
    }
}
impl<P: DerefMut<Target: Unpin>> DerefMut for Pin<P> {
    fn deref_mut(&mut self) -> &mut P::Target {
        Pin::get_mut(Pin::as_mut(self))
    }
}
```

Pin拿住的是一个可以解引用成T的指针类型P, 而不是直接那原本的类型T, 所以对于Pin而言, 你看到的都是`Pin<Box<T>>`, `Pin<&mut T>`, 但不会是`Pin<T>`, 因为Pin的目的是, 把T的内存位置锁住, 从而避免移动后自引用类型带来的引用失效问题

![image-20241223171204932](assets/image-20241223171204932.png)

这样的数据结构就可以正常访问, 但是你无法直接拿到原本的数据结构进而移动它

## 自引用数据结构

当然, 自引用数据结构并非只在异步代码里出现, 只不过异步代码在内部生成用状态机表述Future时, 很容易产生自引用结构, 我们看一个和Future无关的例子:

```rust
#[derive(Debug)]
struct SelfReference {
    name: String,
    // 在初始化后指向 name
    name_ptr: *const String,
}
impl SelfReference {
    pub fn new(name: impl Into<String>) -> Self {
        SelfReference {
            name: name.into(),
            name_ptr: std::ptr::null(),
        }
    }
    pub fn init(&mut self) {
        self.name_ptr = &self.name as *const String;
    }
    pub fn print_name(&self) {
        println!(
            "struct {:p}: (name: {:p} name_ptr: {:p}), name: {}, name_ref: {}"
            self,
            &self.name,
            self.name_ptr,
            self.name,
            // 在使用 ptr 是需要 unsafe
            // SAFETY: 这里 name_ptr 潜在不安全，会指向旧的位置
            unsafe { &*self.name_ptr },
        );
    }
}
fn main() {
    let data = move_creates_issue();
    println!("data: {:?}", data);
    // 如果把下面这句注释掉，程序运行会直接 segment error
    // data.print_name();
    print!("\\n");
    mem_swap_creates_issue();
}
fn move_creates_issue() -> SelfReference {
    let mut data = SelfReference::new("Tyr");
    data.init();
    // 不 move，一切正常
    data.print_name();
    let data = move_it(data);
    // move 之后，name_ref 指向的位置是已经失效的地址
    // 只不过现在 move 前的地址还没被回收挪作它用
    data.print_name();
    data
}
fn mem_swap_creates_issue() {
    let mut data1 = SelfReference::new("Tyr");
    data1.init();
    let mut data2 = SelfReference::new("Lindsey");
    data2.init();
    data1.print_name();
    data2.print_name();
    std::mem::swap(&mut data1, &mut data2);
    data1.print_name();
    data2.print_name();
}
fn move_it(data: SelfReference) -> SelfReference {
    data
}
```

我们创建了一个自引用结构SelfReference, 它里面的name_ref指向了name, 正常使用它的时候, 没有任何问题, 但一旦对这个结构做move操作, name_ref指向的位置还是会move前的name地址, 这就引发了问题

![image-20241223171726407](assets/image-20241223171726407.png)

同样的, 如果我们使用std::mem::swap也会出现类似的问题, 一旦swap, 两个数据的内容交换, 然而, 由于name_ref指向的地址是还是旧的, 所以整个指针体系都混乱了:

![image-20241223171920727](assets/image-20241223171920727.png)

可以看到swap之后, name_ref指向的内容确实和name不一样了, 这就是自引用结构带来的问题

你也许会奇怪, 不是说move也会出问题吗? 为什么第二行打印name_ref还是指向了Tyr? 这是因为move之后, 之前的内存失效, 但是内存地址还没有被挪作它用, 所以还能正常显示Tyr, 这样的内存访问是不安全的, 如果你把main中这句代码注释掉, 程序就会crash

```rust
fn main() {
    let data = move_creates_issue();
    println!("data: {:?}", data);
    // 如果把下面这句注释掉，程序运行会直接 segment error
    // data.print_name();
    print!("\\n");
    mem_swap_creates_issue();
}
```

现在你应该了解到在Rust下, 自引用类型带来的危害了吧

