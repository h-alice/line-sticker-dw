# LINE Sticker Downloader (`line-sticker-dw`)

A command-line tool to download sticker sets from the [LINE Store](https://store.line.me/).

## TL;DR

這是一個示範專案，展示以下功能：

- 使用更輕量化的 `smol` 作為 async runtime，同時用 `reqwest` 操作非同步 HTTP
- 透過 `async-compat` 讓原本只能依賴 `tokio` 的 `reqwest` 也能在 `smol` 上運作
- 使用 `pico-args` 進行 CLI argument 解析，在不需要太多功能的場合下，`pico-args` 更小更輕量
- 使用 `thiserror` 實作屬於這個套件的 error type
- 使用 `tracing` 進行 logging，並加上篩選功能

主要是寫給我自己研究用，如果你無意間發現這個專案，希望會對你在使用這些套件上有幫助！

## Tech stack

- Asynchronous execution: `smol` + `async-compat`
- HTTP client: `reqwest`
- HTML parsing: `scraper`
- CLI argument parsing: `pico-args`
- Error handling: `thiserror`
- Logging: `tracing`

## CLI Usage

```bash
line-sticker-dw <set_id> [base_folder] [options]
```

### Arguments

- `<set_id>`: The ID of the sticker set. You can find this in the URL of the sticker set on the LINE Store (e.g., `https://store.line.me/stickershop/product/1234567` -> ID is `1234567`).
- `[base_folder]`: (Optional) The directory where stickers will be saved. Defaults to a folder named after the `<set_id>`.

### Options

- `-v, --verbose`: Enables debug logging to see more details about the download process.
- `-h, --help`: Prints usage information.

### Example

```bash
# Download sticker set 123456 into a folder named "my_stickers"
line-sticker-dw 123456 my_stickers
```

## License

MIT License - Copyright (c) 2026 Wayne "h-alice" Hong <admin@halice.art>
