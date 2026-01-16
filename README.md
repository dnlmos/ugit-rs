# ugit-rs

Small reimplementation of Git (ugit) written in **Rust**.

It’s based on
[ugit: DIY Git in Python](https://www.leshenko.net/p/ugit/#) by Nikita Leshenko, adapted to explore Git internals and Rust.

> This project is for learning purposes and is not a full Git replacement.

## Notes
`.ugit` directory schema and pointers

```
.ugit/
├── objects/             # actual files (blobs, trees, commits)
└── refs/
    ├── heads/           # branch pointers (direct)
    │   └── master       #      "a1b2c3d4..."
    ├── tags/            # tag pointers (direct)
    │   └── commit13     #      "e5f6g7h8..."
    └── HEAD             # current checkout (symbolic)
                         #      "ref: refs/heads/master"
```
