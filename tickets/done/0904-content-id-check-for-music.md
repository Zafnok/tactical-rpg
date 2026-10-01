---
id: "0904"
title: "Check the chosen music against YouTube Content ID before release"
type: research
milestone: M8 Release
model: sonnet-5
effort: low
status: done
blocked_by: ["0214"]
nick_input: setup
completed: 2026-10-01
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
1. Claude makes one video file per composer: a still picture (a screenshot
   of the credits screen if 0808 is done, otherwise the title screen), with
   that composer's chosen tracks playing one after another.
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

**Closed without doing the check (Nick's decision, 2026-10-01).** Asked
which release the check should come before, Nick said:

> tbh I think we just bite the bullet and if one these guys decides they
> wanna copyright CC-BY stuff then we can swap it out or add a streamer mode
> later...

So no upload test was made and the acceptance criteria above are left
unticked on purpose. The decision is recorded in `docs/design/audio.md`
(*Content ID*). If a streamer or player reports a claim on one of our
tracks, write a ticket then (`write-ticket`): a `00xx` decision for Nick to
pick a replacement track, or a streamer mode that turns the claimed music
off. Nothing waits on this ticket.
