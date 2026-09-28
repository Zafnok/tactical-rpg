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
