# Voices, languages and who writes the script

Decided: 2026-10-03 (the direction; the details are open, see below)
Source: Nick, in conversation; tickets 0042 and 0043 settle the details

## Nick's words

> few high level directives I'm now thinking about. we added CC music, we are
> adding sprites. How about VA? For now I don't have the budget for a VA
> cast. So we can use AI generated with options menu to turn it off, maybe
> even ask upon starting a new game. This is NOT a blocker for ch1 or even
> act 1... but it's something im thinking about. And once we make enough
> money we can hire real VA cast to replace it.
>
> Additionally, I'm thinking, SRPGs seem fairly popular in Japan. So we can
> also add ML translated Japanese option, with human translation later if we
> make enough money as well.

> we should also have the same goals for the script. rn it's fully AI
> generated with minimal input from me bc I don't trust my script writing
> skills. But if we get interest & money I can hire a scriptwriter to make a
> more compelling story. And since our scripts are human readable/writable
> (I hope) then it shouldn't be a huge hassle for them to also add cues for
> i.e. music, portrait transitions, etc.

## What this decides

One idea, three times: **ship a machine-made version now, built so that a
paid person can replace it later without rebuilding anything.**

1. **Voices.** Story lines may be spoken by AI-generated voices. The player
   can turn them off in Options. A hired cast replaces the generated clips
   later, line by line.
2. **Japanese.** The game gets a Japanese language option, translated by
   machine first. A human translation replaces it later.
3. **The script.** Today's script is written by Claude. A hired scriptwriter
   may rewrite it later, working in the same `.dlg` files, cues (music,
   portraits, captions) included.
4. **None of this blocks Chapter 1 or Act 1.**

"AI-generated voices" is an exception Nick is making to his own rule that
shipped art must have had a human work on it (`look-and-feel.md`, ADR-0032
rule 2). It covers voices only; the rule for pictures is unchanged.

## Still open (Nick decides; not yet asked)

Voices, ticket **0043**:

- What is voiced: every story line, only some, or short reactions.
- Whether the lead is voiced (the player names them and picks their gender).
- Each character's voice, picked by ear.
- Whether voices start on or off, and whether a new game asks.
- Which tool makes them, and whether paying for one is fine.
- Whether Japanese text gets Japanese voices.

Japanese, ticket **0042**:

- The Japanese font, picked from rendered screens.
- How the player picks a language (asked on first launch, or only in Options).
- How the game says the translation is machine-made.
- How a Japanese player names the lead.
- Whether names are written in katakana.

Until those are answered, nothing here is a rule a ticket may build on
beyond the four points above.

## How the technical side supports it

- Text in other languages: [ADR-0045](../adr/0045-languages-text-by-key-and-line-ids.md).
- Voice clips: [ADR-0046](../adr/0046-voice-clips-by-line-id.md).
- A human scriptwriter: ticket 0723 (a writer's kit: guide, checker, preview).
- Saying what is machine-made on the credits screen and the store pages:
  ticket 0907.
