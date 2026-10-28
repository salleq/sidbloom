# Licensing and third-party notices

This project is free software, licensed under **GPL-3.0-or-later** (see `LICENSE`).
It is non-commercial and fully open source.

## Third-party components

| Component | License | Use |
|---|---|---|
| [resid-rs](https://github.com/binaryfields/resid-rs) (Rust port of reSID) | GPL-3.0-or-later | SID chip emulation (linked) |
| [reSID](https://github.com/daglem/reSID) by Dag Lem | GPL-2.0-or-later | Original of the above |
| [64tass](https://tass64.sourceforge.net/) | GPL-2.0-or-later | Build tool only, not distributed in the app |
| SID-Wizard by Hermit | "Public Domain" per its SourceForge project page | `.swm` file format reference |

GPL-2.0-or-later and GPL-3.0-or-later are compatible, so combining them under GPL-3.0-or-later is fine.

## Rules for contributors
- No code copied from a project unless its license is verified here first (GPL-compatible or public domain).
- The `.swm` importer is written from the format documentation / file inspection. If any
  SID-Wizard source file is copied verbatim, first check that file's own header: the
  SourceForge project field says public domain, but bundled third-party parts may differ.
- The `.sid` format (PSID/RSID) is an open specification; exported files contain only our
  own player code plus the user's music data.
