# Terminal Clicker

Ein Clicker-Game, das aussieht wie ein Terminal. Desktop-App in Rust mit
[GPUI](https://crates.io/crates/gpui) (`gpui = "=0.2.2"`). Läuft auf macOS.

Schwarz/weiß oder eines von 9 weiteren Themes, eine Monospace-Schrift (JetBrains Mono), keine Rundungen.
Alles lässt sich per Tastatur **und** Maus bedienen.

## Starten

```sh
cargo run --release              # spielen
cargo run -- --admin             # mit Admin-Panel (auch: --debug)
./bundle/make-app.sh             # baut target/release/Terminal Clicker.app
open "target/release/Terminal Clicker.app" --args --admin
cargo test                       # Spiellogik, Animationen, Formatierung – ohne Fenster
```

Beim allerersten Start erscheint nach dem Boot ein kurzes Intro. Dieselbe Seite gibt es
jederzeit im Spiel mit `h` oder über den Reiter `hilfe`.

## Spielen

Bytes sammeln, Upgrades kaufen, mehr Bytes sammeln.

- **Klicken:** `space` oder den Button `[ KLICK ]`. Gedrückt halten ist erlaubt.
- **Upgrades** (`u`): 13 Stück, freigeschaltet nach insgesamt verdienten Bytes. Nur das nächste
  gesperrte Upgrade ist schon lesbar, alles danach bleibt `???`. Die Kosten steigen pro Level.

  | Effekt | Bedeutung | Beispiele |
  | --- | --- | --- |
  | `+B/s` | passives Einkommen | auto_click.sh, cron.d, fork_daemon, mmap.so, … quantum.qpu |
  | `+B/klick` | mehr pro Klick | macro.kbd, hotkey.sh |
  | `×B/klick` | Klickwert vervielfachen | double.bin, overclock.cfg |
  | `×B/s gesamt` | alles Passive vervielfachen | gzip.d |

- **Events:** Ab und zu (alle 45–110 s, nur in der Hauptansicht) ploppt ein Kasten auf. Wer ihn
  innerhalb von 12 s anklickt oder `e` drückt, bekommt:

  | Event | Wirkung |
  | --- | --- |
  | BOOST ×4 | ×4 auf allen Ertrag (Klick und passiv), 30 s |
  | KLICK-RAUSCH ×10 | ×10 pro Klick, 15 s |
  | DATENFUND | sofort 15 % vom Kontostand, höchstens 15 min passives Einkommen |

  Laufende Boni stehen mit Restzeit-Balken unter dem Button. Dasselbe Event nochmal setzt die
  Zeit wieder auf voll.
- **Datenpakete:** Passives Einkommen fliegt als `+451 B fork_daemon` über den Bildschirm. Höchstens
  12 Pakete gleichzeitig, der Rest kommt ins nächste Paket – auch bei riesigem Einkommen bleibt es ruhig.
- **Speichern:** automatisch alle 30 s, nach jedem Kauf und beim Beenden (`q`).

## Tasten

| Taste | Wirkung |
| --- | --- |
| `space` | klicken (Upgrades: kaufen, Settings: umschalten) |
| `u` / `s` / `h` | Upgrades / Settings / Hilfe |
| `e` | Event einsammeln |
| `↑↓` | auswählen |
| `←→` | Wert ändern (Settings, Admin-Zeitraffer) |
| `enter` | starten / kaufen / umschalten / ausführen |
| `esc` | zurück |
| `a` | Admin-Panel (nur mit `--admin`) |
| `q`, `cmd-q` | speichern und beenden |

Die Statuszeile unten zeigt immer die Kürzel der aktuellen Ansicht – sie sind auch klickbar.

## Maus

- **Titelleiste:** eigene Leiste, die macOS-Ampel bleibt nativ. Ziehen auf freien Flächen verschiebt
  das Fenster, Doppelklick zoomt bzw. minimiert (je nach Systemeinstellung). Auf den Reitern wird
  nicht gezogen.
- **Reiter** `main` `upgrades` `settings` `hilfe` (`admin`) wechseln die Ansicht.
- **Boot:** Klick irgendwo startet.
- **Upgrades:** Zeile überfahren wählt aus, Klick kauft.
- **Settings:** Klick = umschalten / nächster Wert, Rechtsklick = vorheriger, `<` `>` gezielt.
  Themes direkt über die Farbkacheln.
- **Events:** Kasten anklicken.

## Settings

Gespeichert in `settings.json`:

- **ascii-hintergrund** an/aus, mit **dichte** und **geschwindigkeit** – Zeichen-Rauschen hinter
  allem, pausiert bei inaktivem Fenster.
- **animationen** an/aus – Zähler-Rollen, Log-Entschlüsseln, Pakete, Button-Effekte, Blinken.
- **zahlen:** `voll` (`10048514 B`), `kb/mb` (`10.05 MB`, 1000er) oder `kib/mib` (`9.58 MiB`, 1024er).
- **theme:** system, hell, dunkel, phosphor, amber, eis, mitternacht, alarm, papier, blaupause.

## Admin-Panel

Nur wenn das Spiel mit `--admin` oder `--debug` gestartet wird – sonst gibt es weder Reiter noch
Kürzel. Dann `a` oder Reiter `admin`:

- Bytes geben (+1.000 bis +1.000.000.000, ×10) – zählt wie verdient, inkl. Meilensteine
- alle Upgrades freischalten, +1 Level auf alle Upgrades
- Event auslösen (zufällig, erscheint in der Hauptansicht)
- Zeitraffer ×1 / ×10 / ×100 / ×1000 für passives Einkommen
- Spielstand zurücksetzen (zweimal bestätigen, Spielzeit bleibt)
- Stats: Kontostand, verdient, Klicks, Raten, Multiplikator, Spielzeit, Einkommen nach Quelle

## Speicherort

`~/Library/Application Support/terminal-clicker/` (`save.json`, `settings.json`).
Zum Testen umlenkbar: `TERMINAL_CLICKER_DIR=/irgendwo cargo run`.

## Struktur

```
src/
├─ main.rs            App starten, Fenster, Tastenbelegung, --admin
├─ game.rs            Spielzustand, Upgrades, Events/Boni, Admin-Funktionen (kennt GPUI nicht)
├─ format.rs          Zahlenformat voll / KB / KiB
├─ theme.rs           Theme-Liste THEMES + „system“
├─ save.rs            JSON laden/speichern, Settings
├─ ascii/
│  ├─ anim.rs         Frame-Animator, Zählwerk-Rollen, Entschlüsseln-Effekt
│  └─ background.rs   3D-Value-Noise-Hintergrund
└─ ui/
   ├─ main_view.rs    Root-View: Ansichten, Aktionen, Loops, Titelleiste, Events, Pakete
   ├─ upgrades.rs     Upgrade-Tabelle
   ├─ settings.rs     Settings-Liste und Theme-Kacheln
   ├─ help.rs         Hilfe / Intro
   ├─ admin.rs        Admin-Panel und Stats
   ├─ log.rs          Log-Panel
   ├─ status_bar.rs   Statuszeile mit klickbaren Kürzeln
   └─ window_drag.rs  Fenster ziehen über AppKit (macOS)
```

## Erweitern

**Neues Upgrade:** in `src/game.rs` an `UPGRADES` anhängen (nach `unlock_at` sortiert, ein Test
prüft das). Level werden per `id` gespeichert, alte Spielstände bleiben gültig. Die Beschreibung
kommt in `description()` in `src/ui/upgrades.rs`.

```rust
up("fusion.core", 5e9, 1.15, 4e9 as u64, Effect::PerSecond(9_000_000.0)),
```

**Neues Theme:** in `src/theme.rs` an `THEMES` anhängen – erscheint automatisch in den Settings.

```rust
ThemeDef {
    id: "teal",         // stabile ID für settings.json
    label: "petrol",    // Name in den Settings
    bg: 0x001414,       // Hintergrund
    fg: 0x5ff5e0,       // Text
    dim: 0x2f8a7e,      // Nebentext
    line: 0x0d3330,     // Trennlinien
},
```

„system“ nimmt `SYSTEM_LIGHT` bzw. `SYSTEM_DARK`. Eine gespeicherte ID, die es nicht mehr gibt,
fällt auf „system“ zurück.

**Neues Event:** `EventKind` in `src/game.rs` erweitern (`title`, `description`, Wirkung in
`trigger_event` / `passive_buff` / `click_buff`) und die Gewichtung in `spawn_event` in
`src/ui/main_view.rs` anpassen.

## Lizenz

Code: [MIT](LICENSE). Die Schrift JetBrains Mono in `assets/fonts/` steht unter der
[SIL Open Font License 1.1](assets/fonts/OFL.txt).
