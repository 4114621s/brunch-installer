# 🔵 Brunch / ChromeOS Installer for Windows

A graphical installer for Windows that enables **dual booting ChromeOS alongside Windows** using [Brunch Framework](https://github.com/sebanc/brunch).

---

## What is this?

**[Brunch Framework](https://github.com/sebanc/brunch)** is an open-source project by [sebanc](https://github.com/sebanc/brunch) that allows running ChromeOS on x86_64 PCs that are not Chromebooks.

**This tool** is a simple Windows GUI that automates the technical installation steps:
- Copies the ChromeOS image file to your disk
- Disables Windows Fast Startup (required for correct dual boot behavior)
- Generates a GRUB2 boot entry for your bootloader

---

## Requirements

| | Requirement |
|---|---|
| **OS** | Windows 10 / 11 (64-bit) |
| **CPU** | Intel 8th Gen+ or AMD Ryzen (supported by Brunch) |
| **Target drive** | NTFS, **BitLocker disabled**, at least 20 GB free |
| **Brunch image** | `.img` or `.bin` file from [Brunch Releases](https://github.com/sebanc/brunch/releases) |
| **Permissions** | Must run as **Administrator** (right-click → Run as administrator) |

---

## Download

[**⬇️ Download Latest Release**](../../releases/latest)

---

## How to use

1. Download a ChromeOS recovery image for your CPU from [cros.tech](https://cros.tech)
2. Download the Brunch Framework from [sebanc/brunch releases](https://github.com/sebanc/brunch/releases)
3. Run `brunch-installer_x64-setup.exe` **as Administrator**
4. Follow the on-screen instructions

---

## ⚠️ Disclaimer

> **Use at your own risk.**
>
> - This is an **independent project** and is not affiliated with Google, the Brunch Framework project, or any other official entity.
> - Brunch is not the intended way for ChromeOS to operate. **Incorrect installation may result in data loss.**
> - Always back up your data before installation.
> - The developers are not responsible for any damage, including data loss, hardware damage, or any other issues arising from the use of this software.
> - Do not install on a machine containing critical or irreplaceable data.

---

## Related projects

- 🔗 [sebanc/brunch](https://github.com/sebanc/brunch) — Brunch Framework (original project)
- 🔗 [cros.tech](https://cros.tech) — Find recovery images by CPU
- 🔗 [r/Brunchbook](https://reddit.com/r/Brunchbook) — Brunch community on Reddit

---

## Build from source

See [`BUILD_INSTRUCTIONS.md`](BUILD_INSTRUCTIONS.md)

---

*This project is not affiliated with Google LLC or ChromeOS.*
