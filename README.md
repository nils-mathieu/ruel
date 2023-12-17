The Ruel Operating System.

# Name

The name Ruel is a temporary code name for the project. It is the name of [Ruel Stroud], a
fictional character from [Wakfu], a French animated series. I like the character and find the
name funny. That's it.

[Ruel Stroud]: https://wakfu.fandom.com/wiki/Ruel_Stroud
[Wakfu]: https://en.wikipedia.org/wiki/Wakfu_(TV_series)

# Goals

This toy operating system project is mostly a learning experience for me. I want to discover
new things, learn how an operating system works, what it does. I also want to have fun.

During my initial research, I found some resources talking about exokernels, which I found
very interesting. I want to explore this concept and see if I can implement the idea in this
project. I might just end up writing yet another microkernel, though :P.

For now, the goal is to focus on the x86_64 architecture. I might attempt to port the project
to ARM later on, but I don't want to get ahead of myself. One instruction set is enough
for now.

# Dependencies

I want to keep the dependencies to a minimum. For two reasons: I want to learn how to do
everything, and avoid relying on other people's code to do the job for me. I also want to
keep the code as simple as possible, and avoid building abstraction layers on top of others
abstraction layers.

# Resources

Surprisingly, there are a *lot* of resources online about operating systems. Here are the one I'm
using often:

- The [OSDev Wiki][wiki] is a great starting point. It has a lot of information about the different
  aspects of operating systems (though maybe a bit too 32-bit centric). It does not replace the
  Intel manuals, but it's generally a good complement when you already know what you want to do
  but need to a refresher on the structures you're working with.

- The [Intel® 64 and IA-32 Architectures Software Developer Manuals][manual] are the official
  documentation for the Intel x86 and x86-64 architectures. They are very detailed and
  comprehensive. They are also very dry and technical. I don't recommend reading them from
  start to finish, but they are a great reference when you need to know how something works
  precisely.

- The [Exokernel whitepaper][whitepaper] is a great read. It's a bit old and I didn't really find
  any more (newer) resources on the subject, but it's a great introduction to the concept of
  exokernels.

[wiki]: https://wiki.osdev.org/Main_Page
[manual]: https://software.intel.com/content/www/us/en/develop/articles/intel-sdm.html
[whitepaper]: https://pdos.csail.mit.edu/6.828/2008/readings/engler95exokernel.pdf

# Trying it at home

## Building

This project uses the latest nightly version of Rust. If you don't have it installed, invoking
the commands described below will automatically install the correct version of Rust for you.

```sh
cargo build --release --target x86_64.json --package ruel
```

This crates a build artifact in `target/x86_64/release/ruel` (if you didn't change the target
directory of Cargo). That is the kernel image.

## Running

The project is not yet ready to run anything. Come back later! :P
