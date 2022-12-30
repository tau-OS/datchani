# Datchani (ดัชนี) - A Certain Magical Indexer
Datchani (not to be confused with a [certain famous artist](https://en.wikipedia.org/wiki/Thawan_Duchanee)) is a filesystem indexer written in Rust. It is designed to be fast, lightweight, and powerful. And as a optimized alternative for the GNOME Tracker.

It is designed to be both a library and a standalone search daemon. You can write frontends for it, or use it as a library as a standalone tool.

Datchani comes with its own search query language, which is inspired by [Google's search query syntax](https://support.google.com/websearch/answer/2466433?hl=en). It is designed to be easy to use, and accessible to everyone.

## Why?
GNOME's Tracker is heavy, slow, bloated, and takes up a lot of resources in the background. We at Fyra Labs wanted to create a new indexer to improve the user experience of tauOS. And we wanted to make it fast, lightweight, and powerful.

The design of Datchani is heavily inspired by Microsoft's scrapped [WinFS](https://en.wikipedia.org/wiki/WinFS) project. Which was an indexer that supports searching for extensive metadata, and file content. Before it was scrapped, it was supposed to be the default indexer for Windows Vista. But it was too ahead of its time, and it was scrapped in favor of a more primitive [Windows Search](https://en.wikipedia.org/wiki/Windows_Search), which was later extended with Bing integration and Cortana in Windows 10-11.

## Name
Datchani is named after the Thai word for "index" (ดัชนี). And yes, it is pronounced "dutch-ah-nee".

We wanted to go for something related to libraries and indexes, and we ran out of ideas. So we just went with Datchani, and it makes for a really fun *Toaru Majutsu no Index* reference. Since you know, the titular nun's name is Index.

> **Note**
> Datchani is still in early development. It is not yet ready for production use.

## Features
- Indexes files and directories
- Ability to tag files and directories
- Advanced querying and filtering
- Ability to search by file content (for supported file types)
- Ability to search by file metadata (file name, file size, file type, etc.)


## TODO
- [ ] Add support for searching by file content
- [x] Add file database
- [ ] Add support for searching by file metadata
- [ ] Add support for tagging files and directories (with tauOS)
- [ ] Merge with `plocate`'s database for faster indexing
