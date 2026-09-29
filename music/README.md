# Music

The game's music tracks (OGG Vorbis), named by `assets/audio/audio.ron`.
They are **not** embedded in the game: this folder ships next to the exe and
next to the WASM on the web, and the game loads a track when it first plays
it ([ADR-0026](../docs/adr/0026-audio-cues-and-music-files.md)).

Only `.ogg` files from here are shipped; this README isn't.
