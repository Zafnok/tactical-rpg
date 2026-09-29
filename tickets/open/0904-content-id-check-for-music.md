---
id: "0904"
title: "Check the chosen music against YouTube Content ID before release"
type: research
milestone: M8 Release
model: sonnet-5
effort: low
status: todo
blocked_by: ["0214"]
nick_input: setup
completed:
---

# 0904 — Check the chosen music against YouTube Content ID

## Context

People who stream or record the game shouldn't get copyright claims for our
music. Some free-music composers register their tracks with YouTube's Content
ID (Scott Buckley, Alexander Nakarada and Vindsvept do, which is why they were
left out in ticket 0020). For the tracks Nick picked
([`docs/design/audio.md`](../../docs/design/audio.md)), research found no
evidence either way. cynicmusic and Alexandr Zhelanov sell or stream their
catalogues, so they're the ones most worth testing. The only reliable test is
uploading.

## Nick input

**Setup** (Claude can't sign in to YouTube):
1. Claude makes one video file per composer: the game's credits screen (0808)
   as the picture, with that composer's chosen tracks playing one after
   another.
2. Nick uploads each at https://studio.youtube.com → **Create** → **Upload
   videos**, with visibility **Private**.
3. After about 10 minutes, open **Content** → the video. If the
   **Restrictions** column says "Copyright claim", click it and note which
   track and claimant.
4. Tell Claude the results, then delete the test videos.

## Scope

**In:** making the test videos, a results table in the Completion notes, and
for any claimed track a `00xx` ticket asking Nick to pick a replacement.

**Out (do not do):** replacing tracks without Nick, or contacting composers.

## Implementation steps

1. Group the music cues in `audio.md` by composer.
2. Render each group to a video. Any free tool is fine (e.g. ffmpeg: a still
   image plus concatenated audio).
3. Hand them to Nick with the steps above; record the results.

## Acceptance criteria

- [ ] Every chosen music track has been through an upload test.
- [ ] Results table in the Completion notes (track → claim / no claim).
- [ ] A replacement decision ticket exists for each claimed track (or none
      were claimed).

## Tests required

- None (manual check).

## Completion notes

*(Filled in by the session that completes the ticket.)*
