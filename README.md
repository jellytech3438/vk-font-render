# Vk-font-render

This project is me learning vulkan-rs which using rust as language to use vulkan api.

## The process

Below shows the process of constructing a alphabet. I first construct the world with rusttype crate which consume .ttf file type as input. After some scaling and sampleing, the cache size of 1000x1000px will be allocated and copy the buffer into it. 

![image](https://github.com/user-attachments/assets/ffefc79a-3e27-40ef-93f5-bd9f730e4254)

## Run

```
$ cargo install
$ cargo run [--release]
```

## Adjust fonts

Currently, there's no CLI or other way to esay modify the text size, color, position. You need to modify _fn queue_text()_ directly.
