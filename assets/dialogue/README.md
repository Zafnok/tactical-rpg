# Dialogue scripts (`.dlg`)

Each `*.dlg` file here holds one or more **scenes**: cutscenes and
conversations with two portraits (left and right), speech and narration.
They are loaded by `trpg_content::dialogue` and validated by the all-assets
test (ADR-0005), so a broken script fails CI with `file:line: message`.
The dialogue screen (ticket 0704) plays them one text box at a time.

## A full example

```
# Chapter 1, opening. Comments are whole lines starting with #.

@scene ch01_opening
@caption Village of Heth, dusk
> The rain had not stopped for three days.
@left  ana neutral
@right bors angry
bors: You're late.
ana[happy]: Better late than... well.
bors: Than never. Say it. I've heard it from you
  often enough to know how it ends.
@right clear
@right mira surprised
mira: Was that Bors? He looked furious.
ana[sad]: He always does.
@end

@scene ch01_after_battle
@left ana neutral
ana: That's the last of them.
@end
```

## Lines

Every line is one of these. Directives start at the very first column.

| Line | Meaning |
| ---- | ------- |
| *(blank)* | Ignored. |
| `# text` | A comment, ignored. Comments take the whole line: `@left ana neutral # hi` is an error. |
| `@scene <id>` | Starts a scene. Every line below belongs to it until `@end`. |
| `@end` | Ends the scene. Every `@scene` needs one. |
| `@caption <text>` | A location/time caption, shown at the top from here until the next `@caption`. |
| `@left <character> <expression>` | Puts a character on the left side with that expression. Whoever stood there leaves. |
| `@right <character> <expression>` | The same, on the right side. |
| `@left clear`, `@right clear` | That side's portrait leaves. |
| `<character>: <text>` | The character speaks. They must be on screen (`@left`/`@right` first). |
| `<character>[<expression>]: <text>` | The character changes expression, then speaks. The new expression stays. |
| `> <text>` | Narration: a text box with no speaker. |
| `  <text>` (indented) | Continues the speech or narration line above; the two are joined with one space. |
| `@choice` … `@endchoice` | The lead's reply choice: see below. |

Each speech or narration line is **one text box** on screen (the screen
splits a long one into pages). Directives (`@caption`, `@left`, `@right`)
take effect before the next text box.

## Rules

The validator reports every broken rule, with the file and line:

- **Ids** (scenes, characters, expressions) are lowercase letters, digits
  and `_`: `ch01_opening`, `bors`, `surprised`.
- **Scene ids** are unique across *all* `.dlg` files; battles and chapters
  refer to scenes by id. Several scenes may share a file.
- **Characters** must exist in `assets/data/characters.ron`.
- **Two portraits at most**: one left, one right. A character can't stand on
  both sides at once.
- **Speakers must be on screen.** Narration needs nobody.
- **Expressions**: a character with a portrait (`assets/portraits/`) may use
  exactly its portrait's expressions; one without may use `neutral`, `happy`,
  `angry`, `sad`, `surprised` (the five every portrait has).
- **Length**: a speech or narration line, continuation lines included, is
  at most **200 characters** (the text box is 3 lines of about 70).
- **Plain ASCII text.** Use `'` and `"`, not curly quotes; `...` not `…`;
  `--` not `—`; `-` not `–`. The error message names the ASCII form.
- Every scene has at least one speech or narration line.

## The lead: reply choices and tokens

The lead is shaped by the player (`docs/design/setting-and-tone.md`,
"Rules for writing the lead"): they speak mostly through **reply choices**,
and the player picks their gender and first name at New Game. The
character id `lead` is the lead; their name plate shows the player's name
and their portrait is `lead_m` or `lead_f` by gender.

### A full example

```
@scene ch01_gate
@left  lead neutral
@right bors angry
bors: {lead}. You took your time.
@choice
* earnest: We do this properly, or not at all.
  bors[surprised]: ...Huh. Fine.
* wry: I've had worse mornings. Not many.
  bors[happy]: Ha! There's the spirit.
  > Even the guards smile.
* blunt: Stop talking. Move.
  bors[angry]: Charming as ever.
@endchoice
> {lead} checks {their} sword. {They} knows the way from here.
lead: Let's move.
@end
```

### Reply choices

| Line | Meaning |
| ---- | ------- |
| `@choice` | Starts a reply choice. |
| `* <tone>: <text>` | One reply, at the very first column. The text is what the lead says; it is shown in the menu (the reply isn't shown again as a text box). The tone (`earnest`, `wry`, `blunt`…) is an id for writers; the player doesn't see it. |
| `  <line>` (two spaces) | A line of that reply's **reaction**: any speech, narration or directive (`@left`, `@caption`…), indented by exactly two spaces. |
| `    <text>` (more spaces) | Continues the reaction's speech or narration line above. |
| `@endchoice` | Ends the choice. Every reply **rejoins** the scene here. |

While the choice is open, the line before it stays in the text box and the
replies are a menu above it. When the reaction ends, each portrait goes
back to the expression it had before the choice (someone who came on during
the reaction shows `neutral`), so every reply leaves the same screen.
Skipping a scene stops at each choice.

### Lead tokens

In speech, narration, captions and reply text:

| Token | Male lead | Female lead |
| ----- | --------- | ----------- |
| `{lead}` | the player's name (default Rowan) | the player's name |
| `{they}` | he | she |
| `{them}` | him | her |
| `{their}` | his | her |
| `{theirs}` | his | hers |
| `{themself}` | himself | herself |

`{They}`, `{Them}`, `{Their}`, `{Theirs}` and `{Themself}` give the
capitalised word, for the start of a sentence. The tokens become he/she, so
write the verb to agree with he/she: `{They} knows`, not `{They} know`.

### Rules for choices and the lead

- A choice has **2 or 3** replies. Choices can't be nested.
- Reply text is at most **60 characters** (it must fit the menu).
- A reaction has at most **4** speech or narration lines, so the scene
  rejoins quickly.
- Every reaction must leave the **same characters on the same sides** (and
  the same caption) as the first reply's does.
- Outside choices the lead speaks only in short, neutral lines: a `lead:`
  line is at most **40 characters**.
- Only the tokens above exist; any other `{...}`, or a `{` without a `}`,
  is an error.
- Lengths count each token at its longest: `{lead}` as 12 characters (the
  longest name), `{themself}` as 7 (`himself`), and so on.
