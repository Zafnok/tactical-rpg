# Third-party assets

Every non-crate third-party item shipped with the game (fonts, vendored
JavaScript, images, audio, SDK files) must be listed here, per
[ADR-0013](docs/adr/0013-licensing-and-third-party-policy.md). Rust crates are
checked automatically by `cargo-deny` and listed in the generated
`THIRD_PARTY_LICENSES.html` in release packages.

Only licenses allowed by ADR-0013 (amended by ADR-0027 and ADR-0032) are
permitted. **Bought art** (ADR-0032) is listed here too, with the seller, the
licence text as quoted on the store page, the date bought, whether it's
AI-assisted, and *private*: its files and licence text live in the private
assets repo (0110), never in this public repo. The
license text must be committed next to the item. The music and sounds are
also listed, with tags, in [`assets/audio/audio.ron`](assets/audio/audio.ron);
the game's credits screen (0808) reads them from there.

**Every row needs a credit with the same source link**: a music or sound
credit in `assets/audio/audio.ron`, or an entry in
[`assets/data/credits.ron`](assets/data/credits.ron) for anything else. The
credits screen shows them all, except entries marked `hidden: true`
(software the player never sees). A test in `trpg-content`
(`every_third_party_asset_has_a_credit`) fails when a row has no credit or
a credit has no row.

## Bought art (private): purchase record

Bought art that isn't in a build yet is recorded here as a list. A pack
gets a row in the table below, and a credit, in the ticket that first puts
it in the game (0706, 0413, 0436, 0437), in the form ticket 0110 sets.

- **Mega Tiles, "2025 Bundle Sale"** (all 37 Mega Tiles products),
  <https://megatiles.itch.io/>. Bought by Nick on **2026-10-02** for
  **$99.99**. *Private*: the files are on Nick's machine and never in this
  repository; the private repository is ticket 0110.
  - **Licence**, the same text in every pack's `License.txt`: "You cannot
    claim ownership of the assets (copyright/IP). Assets can be used both
    in free and commercial games. Assets can be modified freely to fit the
    needs of your game. Redistribution and reselling of the asset files or
    derivatives as is without permission is strictly forbidden."
  - **Character Generator EX 1.2** has its own EULA (Mechwolf Productions):
    the pictures it makes "are owned by the End User and may be used for
    personal, commercial, or non-commercial purposes", but may not be
    distributed or sold "as is" outside a larger project, and the program
    itself may not be redistributed.
  - **Credits named in the packs:** produced and published by Megatiles.
    Portraits and static battlers: Rayane Félix, Lunatic Red. Side-view
    battlers and map sprites: Kodots Games Studio. *Wild Beasts* and other
    battler packs: Lunatic Red, Inazuma. *Epic Monsters* and *Gods and
    Gallants*: Aekashics. Generator software: Mechwolf Productions.
  - **AI-assisted:** no (the store pages say no generative AI was used,
    read 2026-09-30).
  - **What we plan to use:** *Heroes: A New Beginning*, *Heroes 2:
    Rebellious Souls* (faces, still battle pictures, map sprites); still
    battler packs Vol.1–5 with their map sprite packs; *Gods and
    Gallants*, *Epic Monsters*; the Human, Knights, Nobility, Orcs, Dark
    Elves and Beastmen map sprite packs and the Mega Sprite Pack; the World
    Map, Overworld, Dungeons 1 and 2 and Tower tilesets; *Battlebacks
    Vol.1*; the Character Generator. Not planned: the Code Ark sci-fi packs
    and UI kit, *Terrains Plus* and the two *Trees* packs (48-pixel tiles
    in another style).

## Shipped items

| Item | Source URL | Version | License | License file | Used for | Added by ticket |
| ---- | ---------- | ------- | ------- | ------------ | -------- | --------------- |
| Terminus Font (`ter-u16n`, 8×16), converted to `assets/fonts/atlas.png` | https://terminus-font.sourceforge.net/ | 4.49.1 | OFL-1.1 | [`assets/fonts/Terminus-LICENSE.txt`](assets/fonts/Terminus-LICENSE.txt) | The game's only font (every glyph on screen); source BDF in `assets-src/fonts/`, unmodified. The atlas adds four glyphs of our own (`✕ ◯ □ △`, `assets-src/fonts/pad-shapes.bdf`, ticket 0220), which the OFL permits for a modified version not named after the font | 0203 |
| `web/mq_js_bundle.js` (macroquad's JS/WASM loader) | https://github.com/not-fl3/macroquad/blob/5e9b5ca912ac65962c05c0da842a4a70eaae34b9/js/mq_js_bundle.js | 0.4.16 (commit `5e9b5ca9`; no matching git tag, see `web/README.md`) | MIT | [`web/mq_js_bundle-LICENSE-MIT.txt`](web/mq_js_bundle-LICENSE-MIT.txt) | Loads and runs the WASM binary in the browser build | 0206 |
| `web/quad-storage.js` (miniquad `localStorage` JS plugin) | https://github.com/optozorax/quad-storage/blob/3760b953aec17d65cc4ca8edfa39c38e7337ec3a/js/quad-storage.js | 0.1.3 (commit `3760b953`, with a local `version` patch, see `web/README.md`) | MIT | [`web/quad-storage-LICENSE-MIT.txt`](web/quad-storage-LICENSE-MIT.txt) | Backs `trpg_ui::storage::Storage` on web | 0207 |
| `web/sapp_jsutils.js` (JS↔Rust marshalling `quad-storage.js` needs) | https://github.com/not-fl3/sapp-jsutils, shipped inside the `sapp-jsutils` crate's `js/sapp_jsutils.js` | 0.1.7 | MIT (no upstream `LICENSE` file; standard MIT text reconstructed from the crate's declared license, see `web/README.md`) | [`web/sapp_jsutils-LICENSE-MIT.txt`](web/sapp_jsutils-LICENSE-MIT.txt) | Backs `trpg_ui::storage::Storage` on web | 0207 |
| SDL_GameControllerDB (`gamecontrollerdb.txt`), compiled into native builds by the `gilrs` crate | https://github.com/mdqinc/SDL_GameControllerDB, shipped inside the `gilrs` crate (`SDL_GameControllerDB/`) | The copy in `gilrs` 0.11.2 | Zlib | [`crates/app/SDL_GameControllerDB-LICENSE.txt`](crates/app/SDL_GameControllerDB-LICENSE.txt), copied from the crate's `SDL_GameControllerDB/LICENSE` (the `gilrs` crate itself, MIT / Apache-2.0, is covered by `cargo-deny` and `THIRD_PARTY_LICENSES.html`) | Maps PlayStation, Switch and generic controllers to standard button positions (ADR-0034) | 0219 |
| Music: "New Sunrise" by nene | https://opengameart.org/content/new-sunrise | V1 (New Sunrise.wav) for the title screen; V2 (new_sunrise_V2_0.wav) for a first city visit | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `title`, `city_first_visit` cues | 0214 |
| Music: "Squirrel Village" by SoManyWhales | https://opengameart.org/content/squirrel-village | SV_loop.mp3 (the loop file) from squirrelvillage.zip | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `village_home` cue | 0214 |
| Music: "Aria" by Kistol | https://opengameart.org/content/aria | Aria_2.ogg | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `village` cue | 0214 |
| Music: "Dark Forest Theme" by cynicmusic | https://opengameart.org/content/dark-forest-theme | GameMusic_ForestTheme_24_0.mp3 | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `dungeon_tense` cue | 0214 |
| Music: "Dark Quest" by Alexandr Zhelanov | https://opengameart.org/content/dark-quest | Dark Quest.ogg | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `graveyard_desert` cue | 0214 |
| Music: "Father's Scabbard" by SoManyWhales | https://opengameart.org/content/fathers-scabbard | Father's Scabbard - Loop.wav (the loop file) from fathers_scabbard.zip | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `side_quest` cue | 0214 |
| Music: "Field - Orchestra" by migfus20 | https://opengameart.org/content/field-orchestra | asdf_2.mp3 | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `talk_calm` cue | 0214 |
| Music: "Classical Murder" by Alexandr Zhelanov | https://opengameart.org/content/classical-murder | Classical Murder.ogg | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `talk_antagonist` cue | 0214 |
| Music: "Peractorum (from Emotional Orchestral Music)" by César da Rocha | https://opengameart.org/content/emotional-orchestral-music | Peractorum.wav from EmotionalOrchestra_24bit.zip | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `scene_sad` cue | 0214 |
| Music: "Hesitation (Orchestral Version)" by Maarten Schellekens | https://freemusicarchive.org/music/maarten-schellekens/free-music-made-for-screen/hesitation-orchestral-version/ | The orchestral version (the MP3 the page streams; the download button needs a login) | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `scene_tragic` cue | 0214 |
| Music: "Epic Endgame Cinematic" by cynicmusic | https://opengameart.org/content/epic-endgame-cinematic | ChoirChordsBassMaster.wav, the only version (it has vocals; no instrumental exists) | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `mythic_moment` cue | 0214 |
| Music: "Hope (Orchestral battle music)" by MintoDog | https://opengameart.org/content/hopeorchestral-battle-music | hope_orchestral_battle_music_bpm165.flac (made to loop) | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `battle_bright` cue | 0214 |
| Music: "Battle" by mla | https://opengameart.org/content/battle-0 | battle.flac, whole file (the author's loop point, sample 378000, isn't used) | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `battle_bittersweet` cue | 0214 |
| Music: "Sigil" by Kistol | https://opengameart.org/content/sigil | Sigil_3.ogg | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `battle_easy` cue | 0214 |
| Music: "Ending Scene (orchestral version)" by nene | https://opengameart.org/content/ending-scene | ending_scene_orchestral.wav | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `battle_new_area` cue | 0214 |
| Music: "Battle Theme A" by cynicmusic | https://opengameart.org/content/battle-theme-a | battleThemeA.mp3 | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `battle_epic_a` cue | 0214 |
| Music: "Battle Theme B for RPG" by cynicmusic | https://opengameart.org/content/battle-theme-b-for-rpg | battleThemeB.mp3 | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `battle_epic_b` cue | 0214 |
| Music: "Fantasy Choir 2 and 3" by César da Rocha | https://opengameart.org/content/fantasy-choir-3-orchestral-pieces | Fantasy Choir 2.wav and Fantasy Choir 3.wav from FantasyChoir24bit.zip | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `battle_church_2`, `battle_church_3` cues | 0214 |
| Music: "Battle Themes 1, 2, 3 and 5" by Alexandr Zhelanov | https://opengameart.org/content/battle-themes | Battle Theme 1, 2, 3 and 5.mp3 from Battle Themes.zip (not 4) | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `skirmish` pool | 0214 |
| Music: "RPG - Battle Theme" by jocolloman | https://opengameart.org/content/rpg-battle-theme-0 | battle_theme_2.wav (the uncompressed file, made to loop); copyright 2021 Joseph Collard | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `skirmish` pool | 0214 |
| Sound: "Sword sound 1" by Merrick079 | https://freesound.org/s/568170/ | HQ preview | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `hit_sword` cue | 0214 |
| Sound: "Knife Stab" by Mixedupmoviestuff | https://freesound.org/s/179222/ | HQ preview | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `hit_spear`, `hit_axe` cues | 0214 |
| Sound: "Arrow Impact" by Twisted_Euphoria | https://freesound.org/s/205938/ | HQ preview | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `hit_bow` cue | 0214 |
| Sound: "Punch" by EminYILDIRIM | https://freesound.org/s/544680/ | HQ preview | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `hit_gauntlet` cue | 0214 |
| Sound: "Deep Cut / Slash / Gash" by SypherZent | https://freesound.org/s/420674/ | HQ preview | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `crit_physical` cue | 0214 |
| Sound: "Combat Punch Metal Armor" by EminYILDIRIM | https://freesound.org/s/545021/ | HQ preview | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `block` cue | 0214 |
| Sound: "Short-Fireball-Woosh" by wjl | https://freesound.org/s/267887/ | HQ preview | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `cast_fire` cue | 0214 |
| Sound: "Hard Glass Impact" by deleted freesound user 3656686 | https://freesound.org/s/418194/ | HQ preview; the uploader's account was deleted | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `cast_ice` cue | 0214 |
| Sound: "Magic Earth Spell Impact & Punch" by EminYILDIRIM | https://freesound.org/s/541477/ | HQ preview | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `hit_magic` cue | 0214 |
| Sound: "Fireball Impact" by EminYILDIRIM | https://freesound.org/s/577450/ | HQ preview, untrimmed | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `crit_fire` cue | 0214 |
| Sound: "Magic Ice Impact Skill Spell" by EminYILDIRIM | https://freesound.org/s/550267/ | HQ preview, untrimmed | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `crit_ice` cue | 0214 |
| Sound: "Footstep_Dirt_00" by LittleRobotSoundFactory | https://freesound.org/s/270415/ | HQ preview | CC-BY-4.0 | [`assets/audio/licenses/CC-BY-4.0.txt`](assets/audio/licenses/CC-BY-4.0.txt) | `step_foot` cue | 0214 |
| Sound: "dirt/gravel footstep 4" by Yoyodaman234 | https://freesound.org/s/223153/ | HQ preview | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `step_foot` cue | 0214 |
| Sound: "Footstep_Grass_5" by GiocoSound | https://freesound.org/s/421135/ | HQ preview | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `step_foot` cue | 0214 |
| Sound: "Knight Right Footstep on Gravel 5 (With Chainmail)" by Ali_6868 | https://freesound.org/s/384890/ | HQ preview | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `step_armored` cue | 0214 |
| Sound: "Maxheadroom's Galloping Horse" by Podsburgh | https://freesound.org/s/274898/ | HQ preview; one hoof-beat pair cut from 0.895-1.035 s | CC0-1.0 | [`assets/audio/licenses/CC0-1.0.txt`](assets/audio/licenses/CC0-1.0.txt) | `step_mounted` cue | 0214 |
