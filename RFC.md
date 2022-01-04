TiDB Hackathon 2021 RFC: Continuous Perf

Author: Ruihang Xia (@waynexia)

Progress: 都在这个分支上了 ~~也没啥东西~~

Introduce: 提供持续性的 perf record、 perf 结果分析（也许），以及（做的完的话）对 unwind 的加速。

Motivation: It's funny, and 目前 perf 的开销比较小，并且有可能更小。比起之前有想知道的时候才去 perf 的方式，把 perf 一直开着也许能够得到更多的信息。

Design:
- 从 pprof-rs 库入手，包一个持续 perf 的接口来持续记录调用栈。
- 用打表的办法提高 unwinding 速度。
- 用产生的 proto 格式调用栈提供一些分析能力。