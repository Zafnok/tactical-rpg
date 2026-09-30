"""Import the third-party music and sounds chosen in ticket 0020 (ticket 0214).

    python assets-src/audio/import.py            # download (if missing) + convert
    python assets-src/audio/import.py --check    # print loudness of the outputs
    python assets-src/audio/import.py new_sunrise_v1.ogg  # redo only these outputs

Downloads every original into assets-src/audio/originals/ (git-ignored, see
README.md), checks it against the SHA-256 below, then writes the game files:
music to music/, sounds to assets/audio/sfx/. Needs Python 3 and ffmpeg
(MSYS2: `pacman -S mingw-w64-x86_64-ffmpeg`) on PATH or in C:/msys64/mingw64/bin.

Which cue gets which file, and the credits: assets/audio/audio.ron and
docs/design/audio.md.
"""

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import urllib.parse
import urllib.request
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
ORIGINALS = os.path.join(HERE, "originals")
MUSIC_OUT = os.path.join(ROOT, "music")
SFX_OUT = os.path.join(ROOT, "assets", "audio", "sfx")

OGA = "https://opengameart.org/sites/default/files/"
FS = "https://cdn.freesound.org/previews/"

# Music: EBU R128 integrated loudness target, and the true-peak ceiling the
# gain may not push past (then the track stays a little quieter).
MUSIC_LUFS = -20.0
# Sounds are short, so their "integrated" loudness is unreliable: match the
# loudest 400 ms window (EBU momentary max) instead. Louder than the music
# so hits read over it.
SFX_MOMENTARY_LUFS = -18.0
PEAK_CEILING_DBTP = -1.0
# Vorbis quality (-q:a): music ~160 kbps stereo; sounds are tiny either way.
MUSIC_QUALITY = 5
SFX_QUALITY = 6
# The longest a track may be: ADR-0026's memory budget.
MAX_MUSIC_S = 4.5 * 60

# (original file name, URL, SHA-256)
DOWNLOADS = [
    ("New Sunrise.wav", OGA + "New%20Sunrise.wav", "2cdeaf5b65784117e09d10cde1c5a8fb26716751413a35e3b609b928701226d4"),
    ("new_sunrise_V2_0.wav", OGA + "new_sunrise_V2_0.wav", "2e3449c57fa70e1ca12639ce3412050a2a34309674adb264dc07f4d9126902b0"),
    ("squirrelvillage.zip", OGA + "squirrelvillage.zip", "5cb32777fd76b3ddcf8d1cdfe054748b6848fdee457480569d43a6538b6ea9ef"),
    ("Aria_2.ogg", OGA + "Aria_2.ogg", "d46a06457b207664ff2cb24c20791c566e8824d88adb40ae2c510a3a92c3e29b"),
    ("GameMusic_ForestTheme_24_0.mp3", OGA + "GameMusic_ForestTheme_24_0.mp3", "c9e83d01741e97a462c44963aff81645a9c2aa3a7a33e5849114a1399d1b0914"),
    ("Dark Quest.ogg", OGA + "Dark%20Quest.ogg", "4194b82e5fd2cf6cc5067f9a9e17043334e7e892bef9b48b186c4dc75f83775b"),
    ("fathers_scabbard.zip", OGA + "fathers_scabbard.zip", "0e70a1f7b4b77e107c18acb221952ba5acefd183a62a4850df30bc7a884ce7e2"),
    ("asdf_2.mp3", OGA + "asdf_2.mp3", "93d4d441e7ab0a7f994a0031c3501e2655f13fbc1b8480b6935f0582db3843c3"),
    ("Classical Murder.ogg", OGA + "Classical%20Murder.ogg", "af85e82ef1682d262f1c69014eb54f5cfaa6ae0b7b2f8015895c086daf45b4b9"),
    ("EmotionalOrchestra_24bit.zip", OGA + "EmotionalOrchestra_24bit.zip", "fc67c26ac19169d0af8ce0181517254ff998a1f9ed66214e30b974fb9e4a1353"),
    ("ChoirChordsBassMaster.wav", OGA + "ChoirChordsBassMaster.wav", "3cf6f2cb4eb4c22d86da2a5fc6dbada9d9484f57f42e017f5c9b7e6096796bd6"),
    ("hope_orchestral_battle_music_bpm165.flac", OGA + "hope_orchestral_battle_music_bpm165.flac", "5da6876e90fb07cd6e7a28fb03b436bbd1ada4d24b9f6cf6284a2104063c4cc3"),
    ("battle.flac", OGA + "battle.flac", "7b882d4704b6b93f71562567e51fb4469b93b1acb08e10c62ec8fb4e7e86ea5e"),
    ("Sigil_3.ogg", OGA + "Sigil_3.ogg", "fec2ad1be306ee18ca6558fde18d47cb61d9c43d928302ffa0acf7c781cd2d4f"),
    ("ending_scene_orchestral.wav", OGA + "ending_scene_orchestral.wav", "3e0702b3ec4c48eaee852174e2cbb5252655704b47fb716aa9ccb8df7e910a9c"),
    ("battleThemeA.mp3", OGA + "battleThemeA.mp3", "6042399782e581d753d616bc703e66483d5eccb5fb687a20c9a552d68c49e620"),
    ("battleThemeB.mp3", OGA + "battleThemeB.mp3", "8476ce29692d8d317a7d79628af967448ec09d9f66efe692a597336c5cea2629"),
    ("FantasyChoir24bit.zip", OGA + "FantasyChoir24bit.zip", "d2312b968a0b2dca4be7f0414a358b1da3d7fad979514625ba745caf698f520e"),
    ("Battle Themes.zip", OGA + "Battle%20Themes.zip", "8bbbaa8f5f34b157651745e6ef0c1e8c4e8689566f8e7710647174f1563bd714"),
    ("battle_theme_2.wav", OGA + "battle_theme_2.wav", "7cb99c435b24305cc5ca41e18acb64dba3e4a8182a95921f7f97dfa7f1f8f93d"),
    # Free Music Archive's download button needs a login; this is the
    # public file its player streams (320 kbps MP3).
    ("hesitation_orchestral.mp3",
     "https://files.freemusicarchive.org/storage-freemusicarchive-org/tracks/"
     "cublcExin5S6CsrhVDekzAhalIRMesorYx2ufMxv.mp3", "17b17399d97b59ed19ddf8e68817fc887f712c3c93b0e82e62daef6eeb86c096"),
    # freesound originals need a login; these are the public HQ previews,
    # under the same license as the sound (ticket 0214).
    ("freesound/568170_12735833-hq.ogg", FS + "568/568170_12735833-hq.ogg", "57f050ecf50d8652190fef5baafb7c0a0f290beac6ac21e5c0df4a9d567041e4"),
    ("freesound/179222_3337554-hq.ogg", FS + "179/179222_3337554-hq.ogg", "4be3d66ee83d17e91b64bb66055ac167cceddcc14bb694db0777919d4f83cbbf"),
    ("freesound/205938_3842302-hq.ogg", FS + "205/205938_3842302-hq.ogg", "0a2d5b78d4fe7ee755134cd82862695df62dde69cb7ce5cc708382f7d0701173"),
    ("freesound/544680_10912485-hq.ogg", FS + "544/544680_10912485-hq.ogg", "7f97384e13cc4adfb91be47c30648fcc211fc8debcbccb46e1b67283ac7ce8a7"),
    ("freesound/420674_6544098-hq.ogg", FS + "420/420674_6544098-hq.ogg", "ba3fc9df02025d18c27c372384bc11ec93d9d158955d184c97bd3868f80dd917"),
    ("freesound/545021_10912485-hq.ogg", FS + "545/545021_10912485-hq.ogg", "acb2d2f70271a3b131d855629cb6a57c8ef34e5f0285561ae0883d64689e9670"),
    ("freesound/267887_5025429-hq.ogg", FS + "267/267887_5025429-hq.ogg", "2b5cc8efa26bcbf30de4cc81ba21bda01f1de2540b2e7190cf414f25bfa635a2"),
    ("freesound/418194_3656686-hq.ogg", FS + "418/418194_3656686-hq.ogg", "4d0dc21d2128f48afc2b92029e2665cb592c809ed429bad9ec87eae546db42a3"),
    ("freesound/541477_10912485-hq.ogg", FS + "541/541477_10912485-hq.ogg", "4597c7a4ff1cb0491a100f3e66dda509faefbf2116855a67fa44c7fadd1ee5fc"),
    ("freesound/577450_10912485-hq.ogg", FS + "577/577450_10912485-hq.ogg", "dd80a9b7160d27164719ec5b73ce17294a099b67d7b9871048001c15d70803eb"),
    ("freesound/550267_10912485-hq.ogg", FS + "550/550267_10912485-hq.ogg", "f223cb3b0411905f4a7d9c736fcd47a4286b7fdc70fec1e4b72448d4eb918029"),
    ("freesound/270415_5123851-hq.ogg", FS + "270/270415_5123851-hq.ogg", "fda60ed5e5a1a154273e439ba40ca2ad12f04c3d49b4ed5639f2a46bb79c41c4"),
    ("freesound/223153_2792951-hq.ogg", FS + "223/223153_2792951-hq.ogg", "08845c2c1df01a46c66d1a672856ca1ae1adba177a75eb6d6b3377373ed6810e"),
    ("freesound/421135_5820033-hq.ogg", FS + "421/421135_5820033-hq.ogg", "f3f6e0b742917da0754f68cc9cc68beab13ccc3e2ed24a0697cc36c2204afb28"),
    ("freesound/384890_984733-hq.ogg", FS + "384/384890_984733-hq.ogg", "5d20c49236fd1f3cacb3e0408e0a25632a1e98f7836b8cb33d61a6587a48827c"),
    ("freesound/274898_4869300-hq.ogg", FS + "274/274898_4869300-hq.ogg", "d8e6a07ec5f1e9fecbcf2eb17dc66c87ca5b8b8aa6454a0e094ffb716bf7e168"),
]

# (game file in music/, original, file inside the zip or None)
MUSIC = [
    ("new_sunrise_v1.ogg", "New Sunrise.wav", None),
    ("new_sunrise_v2.ogg", "new_sunrise_V2_0.wav", None),
    ("squirrel_village_loop.ogg", "squirrelvillage.zip", "SV_loop.mp3"),
    ("aria.ogg", "Aria_2.ogg", None),
    ("dark_forest_theme.ogg", "GameMusic_ForestTheme_24_0.mp3", None),
    ("dark_quest.ogg", "Dark Quest.ogg", None),
    ("fathers_scabbard_loop.ogg", "fathers_scabbard.zip", "Father's Scabbard - Loop.wav"),
    ("field_orchestra.ogg", "asdf_2.mp3", None),
    ("classical_murder.ogg", "Classical Murder.ogg", None),
    ("peractorum.ogg", "EmotionalOrchestra_24bit.zip", "Peractorum.wav"),
    ("hesitation_orchestral.ogg", "hesitation_orchestral.mp3", None),
    ("epic_endgame_cinematic.ogg", "ChoirChordsBassMaster.wav", None),
    ("hope_orchestral_battle.ogg", "hope_orchestral_battle_music_bpm165.flac", None),
    ("battle_mla.ogg", "battle.flac", None),
    ("sigil.ogg", "Sigil_3.ogg", None),
    ("ending_scene_orchestral.ogg", "ending_scene_orchestral.wav", None),
    ("battle_theme_a.ogg", "battleThemeA.mp3", None),
    ("battle_theme_b.ogg", "battleThemeB.mp3", None),
    ("fantasy_choir_2.ogg", "FantasyChoir24bit.zip", "Fantasy Choir 2.wav"),
    ("fantasy_choir_3.ogg", "FantasyChoir24bit.zip", "Fantasy Choir 3.wav"),
    ("zhelanov_battle_theme_1.ogg", "Battle Themes.zip", "Battle Theme 1.mp3"),
    ("zhelanov_battle_theme_2.ogg", "Battle Themes.zip", "Battle Theme 2.mp3"),
    ("zhelanov_battle_theme_3.ogg", "Battle Themes.zip", "Battle Theme 3.mp3"),
    ("zhelanov_battle_theme_5.ogg", "Battle Themes.zip", "Battle Theme 5.mp3"),
    ("rpg_battle_theme.ogg", "battle_theme_2.wav", None),
]

# Extra ffmpeg filters for a music track, applied before loudness matching.
MUSIC_CUTS = {
    # The original sits below -60 dB for its first 3 s before the fade-in
    # (ticket 0221): the title screen seemed silent. Keep the fade, drop the
    # silence; a 50 ms fade-in avoids a click at the cut.
    "new_sunrise_v1.ogg": "atrim=start=3.0,asetpts=PTS-STARTPTS,afade=t=in:d=0.05",
}

# (game file in assets/audio/sfx/, original, extra ffmpeg filters before
# the leading-silence trim, or None)
SFX = [
    ("sword_sound_1.ogg", "freesound/568170_12735833-hq.ogg", None),
    ("knife_stab.ogg", "freesound/179222_3337554-hq.ogg", None),
    ("arrow_impact.ogg", "freesound/205938_3842302-hq.ogg", None),
    ("punch.ogg", "freesound/544680_10912485-hq.ogg", None),
    ("deep_cut.ogg", "freesound/420674_6544098-hq.ogg", None),
    ("combat_punch_metal_armor.ogg", "freesound/545021_10912485-hq.ogg", None),
    ("short_fireball_woosh.ogg", "freesound/267887_5025429-hq.ogg", None),
    ("hard_glass_impact.ogg", "freesound/418194_3656686-hq.ogg", None),
    ("magic_earth_spell_impact.ogg", "freesound/541477_10912485-hq.ogg", None),
    # Nick: the magic crits stay whole ("dont need trim").
    ("fireball_impact.ogg", "freesound/577450_10912485-hq.ogg", None),
    ("magic_ice_impact.ogg", "freesound/550267_10912485-hq.ogg", None),
    ("footstep_dirt_00.ogg", "freesound/270415_5123851-hq.ogg", None),
    ("footstep_dirt_gravel_4.ogg", "freesound/223153_2792951-hq.ogg", None),
    ("footstep_grass_5.ogg", "freesound/421135_5820033-hq.ogg", None),
    ("knight_footstep_gravel_5.ogg", "freesound/384890_984733-hq.ogg", None),
    # One hoof-beat pair (0.895-1.035 s of the 39 s gallop), faded out: a
    # unit walks 12 tiles/s (crates/ui/src/screens/battle/mode.rs,
    # WALK_TILES_PER_S), so a step sound fires every ~83 ms.
    ("galloping_horse_step.ogg", "freesound/274898_4869300-hq.ogg",
     "atrim=start=0.895:end=1.035,asetpts=PTS-STARTPTS,afade=t=out:st=0.105:d=0.035"),
]


def ffmpeg_exe(name):
    found = shutil.which(name)
    if found:
        return found
    msys = os.path.join("C:/msys64/mingw64/bin", name + ".exe")
    if os.path.exists(msys):
        return msys
    sys.exit(f"{name} not found: install ffmpeg (see this file's docstring)")


FFMPEG = ffmpeg_exe("ffmpeg")


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def download(only):
    """Fetch the originals (only those `only`'s outputs need, if given)."""
    needed = {orig for out, orig, _ in MUSIC + SFX if out in only}
    for name, url, digest in DOWNLOADS:
        if only and name not in needed:
            continue
        path = os.path.join(ORIGINALS, name)
        if not os.path.exists(path):
            print("download", url)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req) as r, open(path + ".part", "wb") as f:
                shutil.copyfileobj(r, f)
            os.replace(path + ".part", path)
        got = sha256(path)
        if got != digest:
            sys.exit(f"{name}: SHA-256 {got}, expected {digest} (the source changed?)")


def source_path(original, member):
    """The file to feed ffmpeg: the original, or a member unpacked from it."""
    path = os.path.join(ORIGINALS, original)
    if member is None:
        return path
    out = os.path.join(ORIGINALS, "unzipped", member)
    if not os.path.exists(out):
        os.makedirs(os.path.dirname(out), exist_ok=True)
        with zipfile.ZipFile(path) as z, open(out, "wb") as f:
            f.write(z.read(member))
    return out


def run(args):
    return subprocess.run(
        [FFMPEG, "-hide_banner", "-nostats", *args],
        check=True, capture_output=True, text=True, encoding="utf-8",
    ).stderr


def loudness(path, filters):
    """(integrated LUFS, max momentary LUFS, true peak dBTP) after `filters`.
    Padded to 0.5 s of silence so a short sound fills a 400 ms window."""
    chain = ",".join(f for f in [filters, "apad=whole_dur=0.5", "ebur128=peak=true:framelog=info"] if f)
    log = run(["-i", path, "-af", chain, "-f", "null", "-"])
    summary = log[log.rindex("Summary:"):]
    integrated = float(re.search(r"I:\s+(-?[\d.]+|-inf) LUFS", summary).group(1))
    peak = float(re.search(r"Peak:\s+(-?[\d.]+|-inf) dBFS", summary).group(1))
    momentary = max(float(m) for m in re.findall(r"\bM:\s*(-?[\d.]+)", log[:log.rindex("Summary:")]))
    return integrated, momentary, peak


def gain_for(level, target, peak):
    return min(target - level, PEAK_CEILING_DBTP - peak)


def convert_music(only):
    os.makedirs(MUSIC_OUT, exist_ok=True)
    for out, original, member in MUSIC:
        if only and out not in only:
            continue
        src = source_path(original, member)
        if duration(src) > MAX_MUSIC_S:
            sys.exit(f"{out}: longer than {MAX_MUSIC_S / 60} minutes (ADR-0026 memory budget)")
        base = ",".join(f for f in [MUSIC_CUTS.get(out), "aresample=44100:resampler=soxr"] if f)
        integrated, _, peak = loudness(src, base)
        gain = gain_for(integrated, MUSIC_LUFS, peak)
        dest = os.path.join(MUSIC_OUT, out)
        run(["-y", "-i", src, "-map_metadata", "-1", "-vn", "-af", f"{base},volume={gain:.2f}dB",
             "-ac", "2", "-ar", "44100", "-c:a", "libvorbis", "-q:a", str(MUSIC_QUALITY), dest])
        print(f"music {out:32} I {integrated:6.1f} LUFS, peak {peak:5.1f} dBTP, gain {gain:+5.1f} dB")


def convert_sfx(only):
    os.makedirs(SFX_OUT, exist_ok=True)
    for out, original, extra in SFX:
        if only and out not in only:
            continue
        src = source_path(original, None)
        # Mono, 44.1 kHz, then drop the silence before the sound starts.
        base = ",".join(f for f in [
            extra,
            "pan=mono|c0=0.5*c0+0.5*c1" if channels(src) == 2 else None,
            "aresample=44100:resampler=soxr",
            "silenceremove=start_periods=1:start_threshold=-50dB",
        ] if f)
        _, momentary, peak = loudness(src, base)
        gain = gain_for(momentary, SFX_MOMENTARY_LUFS, peak)
        dest = os.path.join(SFX_OUT, out)
        run(["-y", "-i", src, "-map_metadata", "-1", "-af", f"{base},volume={gain:.2f}dB",
             "-ac", "1", "-ar", "44100", "-c:a", "libvorbis", "-q:a", str(SFX_QUALITY), dest])
        print(f"sfx   {out:32} M {momentary:6.1f} LUFS, peak {peak:5.1f} dBTP, gain {gain:+5.1f} dB")


def probe(path, entries):
    ffprobe = FFMPEG.replace("ffmpeg", "ffprobe")
    out = subprocess.run([ffprobe, "-v", "error", "-select_streams", "a:0", "-show_entries",
                          entries, "-of", "json", path],
                         check=True, capture_output=True, text=True).stdout
    return json.loads(out)


def channels(path):
    return probe(path, "stream=channels")["streams"][0]["channels"]


def duration(path):
    return float(probe(path, "format=duration")["format"]["duration"])


def check():
    for folder, target in [(MUSIC_OUT, "I"), (SFX_OUT, "M")]:
        for name in sorted(os.listdir(folder)):
            if name.endswith(".ogg"):
                i, m, p = loudness(os.path.join(folder, name), None)
                print(f"{name:34} I {i:6.1f}  M max {m:6.1f}  peak {p:5.1f}  (target {target})")


def main():
    if "--check" in sys.argv:
        check()
        return
    only = {a for a in sys.argv[1:] if not a.startswith("-")}
    known = {m[0] for m in MUSIC} | {s[0] for s in SFX}
    if only - known:
        sys.exit(f"unknown output(s): {', '.join(sorted(only - known))}")
    download(only)
    convert_music(only)
    convert_sfx(only)


if __name__ == "__main__":
    main()
