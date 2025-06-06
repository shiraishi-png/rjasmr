> **Note:** This is a fork of the original [rjasmr](https://github.com/SophiaH67/rjasmr) by [SophiaH67](https://github.com/SophiaH67). This version includes some enhancements, including download progress bars, automatic thumbnail embedding, and more robust error handling.
A huge thank you to SophiaH67 for creating the original program that this work is based on.

Original description from the author:
# RJASMR

Simple [japaneseasmr.com](https://japaneseasmr.com/) downloader, written in Rust🚀.

## Usage

```bash
rjasmr <link1> <link2> ...
```

## Installation

```bash
cargo install --git https://github.com/marnixah/rjasmr.git
```

## New Features in this Fork

This version of `rjasmr` adds the following improvements:

*   **Visual Download Progress:** Downloads now feature a real-time progress bar, showing speed, downloaded size, and estimated time remaining.
*   **Automatic Thumbnail Embedding:** The script now scrapes the page for the cover art, downloads it, and automatically embeds it into the downloaded `.mp3` files.
*   **Robust Error Handling:** The program no longer panics or crashes on network errors or incomplete files. It reports the error and continues gracefully.
*   **Download Verification:** The script now verifies that the downloaded file size matches the expected size from the server, ensuring you don't get incomplete files.
*   **Smarter Scraping:** The logic for finding thumbnails has been improved to handle multiple different page layouts on the target website.

## Usage

The core usage remains the same. Pass one or more URLs to the program as command-line arguments.

```bash
rjasmr <link1> <link2> ...
```
