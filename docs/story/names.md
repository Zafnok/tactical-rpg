# Name registry

Every proper noun in the story has a **stable id** here. Nick isn't sold on
the names yet (gate 1, 2026-09-28: "I hope you set them all as variables in
some way"), so **the ids are what's fixed; the display names can change at
any time.**

## How renaming works

- **Game scripts and data never write a registered name literally.** They use
  the id: speaker ids in `.dlg` files are the character ids below, and names
  inside text are name tokens resolved from the game's names table (format
  and token syntax: `assets/dialogue/README.md`, "Names"). Renaming in the
  game = one line in that table, `assets/data/names.ron` (it holds every id
  below; a test keeps the two in step).
- **The story docs** (`bible.md`, `characters/`, `outline.md`, `chapters/`,
  `ledger.md`) use the display names so they stay readable. To rename: change
  the name in this file, then replace the old name across `docs/story/` in the
  same commit. Every name below is unique, so a whole-word find-and-replace is
  safe (check `Rue`, `Wren`, `Crane`, `Rook`, `Holt`, `Mast` and `Piers`, which
  are also ordinary English words, by eye).
- **A name's short forms are separate rows** (the "Short forms" table below:
  first names, surnames, family names, a god's name and title on their own), because
  people mostly say "Hollis", not "Hollis Marr". **Renaming a character means
  changing the full name and every short form of it** (rename Hollis Marr and
  `retainer`, `retainer.first` and, if the family name changes, `family.marr`
  and the other Marrs all change), here and in `names.ron`. Nothing derives
  one from the other, so a short form can also be a nickname that isn't part
  of the full name.
- **Character sheet files are named by id** (`characters/retainer.md`), not by
  name, so they never move.
- New names get an id here first.

## Characters

| Id | Current name | Who |
| -- | ------------ | --- |
| `lead` | *(player's choice)*, default **Ellery** · family name **Veyne** | The lead. First name chosen by the player; the family name is fixed text (most people just say "Veyne") |
| `retainer` | Hollis Marr | Old captain of House Veyne's guard; Dace's and Wren's father |
| `sergeant` | Tamsin Rook | Cavalry sergeant of the Unpaid |
| `poacher` | Aske | Brennish refugee archer |
| `keeper` | Piers | The Vigil's keeper of Harrowby |
| `heretic` | Rue | Self-vowed fire mage, former Candle |
| `rival` | Dace Marr | The lead's best friend; the king's Hound |
| `king` | Emeric | King of Ardeval; the antagonist |
| `vowmaster` | Absalom Crane | The Vigil's Master of Vows |
| `red_captain` | Harl Coster ("Red Harl") | Captain of the Red Company; Chapter 1 boss |
| `sister` | Wren Marr | Dace's younger sister, missing |
| `prince` | Aurel | King Emeric's son, died at the Ashfields |
| `vosse` | Harrick Vosse | The king's man in the Thornmarch, killed four years ago |

Later Act 1 recruits (named in `outline.md`, sheets written by later tickets):

| Id | Current name | Who |
| -- | ------------ | --- |
| `defector` | Joss Pellam | Young Hound who defects at Kell's Ford |
| `prizefighter` | Gil Parrow | Travelling prize-fighter, "Champion of Six Fairs" |
| `shieldbearer` | Hedda Ravn | The Brennish envoy's bodyguard |
| `envoy` | Ragna Holt | Brennish envoy in Saltmere (NPC) |
| `battlemage` | Oriel Mast | Crown battle-mage who gave the order at the Ashfields |

## Short forms

What people say instead of the full name. Scripts use these ids like any
other (`{n:retainer.first}`); the validator rejects a short form written out
just as it does a full name.

| Id | Current short form | Short for |
| -- | ------------------ | --------- |
| `retainer.first` | Hollis | `retainer` (Hollis Marr) |
| `sergeant.first` | Tamsin | `sergeant` (Tamsin Rook) |
| `sergeant.last` | Rook | `sergeant` (Tamsin Rook): what Harl calls her |
| `rival.first` | Dace | `rival` (Dace Marr) |
| `vowmaster.first` | Absalom | `vowmaster` (Absalom Crane) |
| `vowmaster.last` | Crane | `vowmaster` (Absalom Crane) |
| `vowmaster.title` | the Master of Vows | `vowmaster`: his office in the Vigil, for people who don't say his name |
| `red_captain.first` | Harl | `red_captain` (Harl Coster) |
| `red_captain.last` | Coster | `red_captain` (Harl Coster) |
| `red_captain.nickname` | Red Harl | `red_captain`: what the Thornmarch calls him |
| `sister.first` | Wren | `sister` (Wren Marr) |
| `vosse.first` | Harrick | `vosse` (Harrick Vosse) |
| `vosse.last` | Vosse | `vosse` (Harrick Vosse) |
| `defector.first` | Joss | `defector` (Joss Pellam) |
| `defector.last` | Pellam | `defector` (Joss Pellam) |
| `prizefighter.first` | Gil | `prizefighter` (Gil Parrow) |
| `prizefighter.last` | Parrow | `prizefighter` (Gil Parrow) |
| `shieldbearer.first` | Hedda | `shieldbearer` (Hedda Ravn) |
| `shieldbearer.last` | Ravn | `shieldbearer` (Hedda Ravn) |
| `envoy.first` | Ragna | `envoy` (Ragna Holt) |
| `envoy.last` | Holt | `envoy` (Ragna Holt) |
| `battlemage.first` | Oriel | `battlemage` (Oriel Mast) |
| `battlemage.last` | Mast | `battlemage` (Oriel Mast) |
| `family.marr` | Marr | The family name of `retainer`, `rival` and `sister` |
| `family.veyne` | Veyne | The lead's family name (also in `house.veyne` and `place.veyne_hall`) |
| `faction.brennmark.adj` | Brennish | `faction.brennmark` without its article: the adjective ("a Brennish bow"), and what Tamsin calls Aske |
| `god.mother.name` | Ama | `god.mother` (Ama, the Mother) |
| `god.mother.title` | the Mother | `god.mother` |
| `god.pyre.name` | Vael | `god.pyre` (Vael, the Pyre) |
| `god.pyre.title` | the Pyre | `god.pyre` |
| `god.winter.name` | Hrim | `god.winter` (Hrim, the Long Winter) |
| `god.winter.title` | the Long Winter | `god.winter` |
| `god.hand.name` | Orun | `god.hand` (Orun, the Hand) |
| `god.hand.title` | the Hand | `god.hand` |
| `god.door.name` | Othe | `god.door` (Othe, the Doorkeeper) |
| `god.door.title` | the Doorkeeper | `god.door` |

## Places

| Id | Current name | What |
| -- | ------------ | ---- |
| `place.west` | the Old Kingdoms | The home continent (Act 1 and Act 3) |
| `place.east` | the Jade Reach | The eastern, wuxia-inspired continent (Act 2) |
| `place.south` | the Sunward Isles | A southern land nobody in the story visits yet |
| `place.ardeval` | Ardeval | The lead's kingdom |
| `place.capital` | Varenhall | Ardeval's capital |
| `place.brennmark` | Brennmark | The northern kingdom Ardeval fought |
| `place.thornmarch` | the Thornmarch | Ardeval's poor border march; the lead's place of exile |
| `place.harrowby` | Harrowby | The Thornmarch hamlet where the lead lives |
| `place.veyne_hall` | Veyne Hall | House Veyne's old seat, now Dace's |
| `place.ashfields` | the Ashfields | The battlefield where the war ended |
| `place.ossary` | Ossary | The Vigil's holy city |
| `place.greywater` | Greywater Abbey | The Vigil's house for the Thornmarch |
| `place.saltmere` | Saltmere | Chief port of the southern league |
| `place.kells_ford` | Kell's Ford | River village on the Thornmarch's edge |
| `place.millhaven` | Millhaven | Market town in southern Ardeval |
| `place.cairnford` | Cairnford | Border town near the Ashfields |

## Factions, houses and groups

| Id | Current name | What |
| -- | ------------ | ---- |
| `faction.crown` | the Crown of Ardeval | The king's court and army |
| `faction.hounds` | the Hounds | The king's enforcers, led by Dace |
| `faction.vigil` | the Vigil | The church that keeps the dead gods' vows |
| `faction.brennmark` | the Brennish (Crown of Brennmark) | The northern kingdom's people and crown |
| `faction.unpaid` | the Unpaid | Veterans the crown never paid |
| `faction.red_company` | the Red Company | Harl's company of the Unpaid |
| `faction.league` | the Saltmere League | The southern merchant ports |
| `house.veyne` | House Veyne | The lead's family |
| `group.candles` | the Candles | The Vigil's child battle-mages |

## Gods and magic terms

| Id | Current name | What |
| -- | ------------ | ---- |
| `god.mother` | Ama, the Mother | Dead goddess of mending (healing vows) |
| `god.pyre` | Vael, the Pyre | Dead god of hearth and pyre (fire vows) |
| `god.winter` | Hrim, the Long Winter | Dead god of frost (frost vows) |
| `god.hand` | Orun, the Hand | Dead god of force (force vows) |
| `god.door` | Othe, the Doorkeeper | Dead god of death (forbidden) |
| `term.vow` | a vow | A sworn spell |
| `term.gifted` | the gifted | People born able to swear vows |
| `term.door` | the Door | Othe's door between the living and the dead |
| `term.door_vow` | the Door Vow | The forbidden vow that opens it |
| `shuyi` | **Shuyi** *(fixed: it's in the game's title, so never rename it)* | The Jade Reach's long-unseen guardian of the lifeblood they worship. Many people use the name for the lifeblood itself. See `docs/design/title.md` |
| `term.echo` | an echo | A vow still burning with no caster (elementals) |
| `term.unfinished` | the Unfinished | Those who come back through the Door incomplete |
| `term.final_vow` | the Final Vow | The vow to lay down steel (tier-3 casters) |
| `term.long_dusk` | the Long Dusk | The age in which the gods died |
| `term.breath` | breath | The East's own discipline instead of vows |
| `event.border_war` | the Border War | Ardeval against Brennmark, 13 to 3 years ago |
| `event.ash_peace` | the Ash Peace | The treaty that ended it |

## Minor names

| Id | Current name | What |
| -- | ------------ | ---- |
| `brother_tor` | Tor | Aske's older brother, burned at the Ashfields |
| `messenger` | Messenger | A Vigil messenger with no name of his own: the name plate in Chapter 1's last scene |
| `place.coldwell` | Coldwell | The farm the Red Company burned last winter |
| `place.aldwater` | Aldwater Toll Bridge | Chapter 4 fixed skirmish |
| `place.millhaven_road` | Millhaven Road | Chapter 4 fixed skirmish |
| `place.salt_road` | the Salt Road | Chapter 5 fixed skirmish |
| `place.smugglers_cove` | Smugglers' Cove | Chapter 5 fixed skirmish |
| `place.burnt_mill` | the Burnt Mill | Chapter 6 fixed skirmish |
| `place.pilgrims_road` | Pilgrims' Road | Chapter 7 fixed skirmish |
| `place.ossuary_gate` | the Ossuary Gate | Chapter 7 fixed skirmish |
| `word.mother` | *mor* | Brennish for mother (Aske) |
| `word.home` | *hus* | Brennish for home (Aske) |
