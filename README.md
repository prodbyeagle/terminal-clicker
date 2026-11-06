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
- **Upgrades** (`u`): 30 Stück, freigeschaltet nach insgesamt verdienten Bytes. Nur das nächste
  gesperrte Upgrade ist schon lesbar, alles danach bleibt `???`. Die Kosten steigen pro Level.

  | Effekt | Bedeutung | Beispiele |
  | --- | --- | --- |
  | `+B/s` | passives Einkommen | auto_click.sh, cron.d, kernel.ko, … beyond.null |
  | `+B/klick` | mehr pro Klick | macro.kbd, hotkey.sh |
  | `×B/klick` | Klickwert vervielfachen | double.bin, overclock.cfg, neural.link |
  | `×B/s gesamt` | alles Passive vervielfachen | gzip.d, ai.model, singularity.ai |
  | `krit-chance` | Chance (max. 50 %) auf einen Klick ×10 | lucky.bit |
  | `klick +s B/s` | jeder Klick bringt Sekunden passives Einkommen | pipe.sh |
  | `events öfter & länger` | kürzerer Abstand, längere Boni | notify.d |
  | `offline` | verdient bei geschlossenem Spiel weiter (max. 8 h) | systemd.service |
  | `-kosten` | alle Upgrades billiger (bis höchstens −75 %) | compiler.o3 |

- **Events:** Ab und zu (alle 45–110 s, mit `notify.d` öfter, nur in der Hauptansicht) ploppt ein
  Kasten auf. Wer ihn innerhalb von 12 s anklickt oder `e` drückt, bekommt:

  | Event | Wirkung |
  | --- | --- |
  | BOOST ×4 | ×4 auf allen Ertrag (Klick und passiv), 30 s |
  | KLICK-RAUSCH ×10 | ×10 pro Klick, 15 s |
  | DATENFUND | sofort 15 % vom Kontostand, höchstens 15 min passives Einkommen |
  | KETTENREAKTION | jeder Klick bringt zusätzlich 10 s passives Einkommen, 15 s |
  | RABATT -50 % | alle Upgrades zum halben Preis, 30 s |
  | GLITCH ??? | Glücksspiel: 65 % OVERDRIVE ×7 auf alles (20 s), sonst LAG ×0.5 (10 s) |
  | DDOS-ANGRIFF | Bedrohung (gestrichelter Kasten): abwehren = +60 s Einkommen, ignorieren = LAG ×0.5 B/s für 30 s |

  Laufende Boni stehen mit Restzeit-Balken unter dem Button. Dieselbe Wirkung nochmal setzt die
  Zeit wieder auf voll.
- **Datenpakete:** Passives Einkommen fliegt als `+451 B fork_daemon` über den Bildschirm. Höchstens
  12 Pakete gleichzeitig, der Rest kommt ins nächste Paket – auch bei riesigem Einkommen bleibt es ruhig.
- **Speichern:** automatisch alle 30 s, nach jedem Kauf und beim Beenden (`q`).
- **Kein Limit:** Alle Mengen sind `f64` – es geht weit über 18,45 EB (`u64::MAX`) hinaus, bis etwa 10^308.

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

- **ascii-hintergrund** an/aus – Zeichen-Muster hinter allem, pausiert bei inaktivem Fenster.
  Einstellbar:
  - **muster:** wolken (Rauschen), wellen, plasma, ringe, regen (fallende Spalten), sterne (Funkeln)
  - **zeichen:** klassisch (`.:-=+*#`), blöcke (`░▒▓█`), binär (`01`), punkte (`·•●`), zahlen (`1–9`)
  - **dichte**, **größe** (fein/normal/grob), **deckkraft** (hauch bis stark), **geschwindigkeit**
- **animationen** an/aus – Zähler-Rollen, Log-Entschlüsseln, Pakete, Button-Effekte, Blinken.
- **zahlen:** `voll` (`10048514 B`), `kb/mb` (`10.05 MB`, 1000er, bis QB) oder `kib/mib`
  (`9.58 MiB`, 1024er, bis QiB). Ganz große Werte werden wissenschaftlich geschrieben (`1.23e35 B`).
- **theme:** system, hell, dunkel, phosphor, amber, eis, mitternacht, alarm, papier, blaupause.

## Admin-Panel

Nur wenn das Spiel mit `--admin` oder `--debug` gestartet wird – sonst gibt es weder Reiter noch
Kürzel. Dann `a` oder Reiter `admin`:

- Bytes geben (+1e3, +1e6, +1e9, +1e15, +1e21, ×10) – zählt wie verdient, inkl. Meilensteine
- alle Upgrades freischalten, +1 Level auf alle Upgrades
- Event auslösen – zufällig oder gezielt (`←→` wählt das Event), erscheint in der Hauptansicht
- Zeitraffer ×1 / ×10 / ×100 / ×1000 für passives Einkommen
- Spielstand zurücksetzen (zweimal bestätigen, Spielzeit bleibt)
- Stats: Kontostand, verdient, Klicks, Raten, Multiplikatoren, Krit-Chance, Offline-Anteil,
  Kosten-Faktor, Event-Abstand, Spielzeit, Einkommen nach Quelle

## Speicherort

`~/Library/Application Support/terminal-clicker/` (`save.json`, `settings.json`).
Zum Testen umlenkbar: `TERMINAL_CLICKER_DIR=/irgendwo cargo run`.

## Struktur

```
src/
├─ main.rs            Schrift, Tastenbelegung, Fenster öffnen, --admin
├─ game/              Spiellogik ohne UI – komplett per `cargo test` prüfbar
│  ├─ mod.rs          Spielstand, Klicken, Einkommen, Kaufen, Offline, Admin-Funktionen
│  ├─ upgrades.rs     Upgrade-Katalog (Kosten, Effekt, Beschreibung)
│  └─ events.rs       Events, Boni und der Taktgeber, der sie auftauchen lässt
├─ settings.rs        Einstellungen (typisierte Werte, kompatibel zu alten Dateien)
├─ format.rs          Zahlen: voll / KB / KiB, Tausenderpunkte, Dauer, Balken
├─ theme.rs           Theme-Liste THEMES + „system“
├─ save.rs            JSON laden/speichern; defekte Dateien werden beiseitegelegt
├─ rng.rs             ein Zufallsgenerator für Spiel und Deko
├─ ascii/             Zeichen-Effekte: Animator, Zählwerk, Entschlüsseln, Hintergrund
└─ ui/
   ├─ app.rs          verarbeitet jede Eingabe an einer Stelle, Takt, Rahmen
   ├─ msg.rs          alles, was der Spieler tun kann, als `Msg`
   ├─ keys.rs         die Tastenbelegung – einmal (Bindungen, Statuszeile, Hilfe)
   ├─ page.rs         was jede Ansicht kann (die App kennt nur dieses Interface)
   ├─ session.rs      gemeinsamer Zustand: Spiel, Settings, Log, Events, Animationen
   ├─ list.rs         gemeinsame Bedienung aller Listen-Ansichten
   ├─ boot.rs · main_screen.rs · upgrades.rs · settings.rs · help.rs · admin.rs   die Ansichten
   ├─ packets.rs      Datenpakete (passives Einkommen sichtbar)
   ├─ fx.rs           die Effekte, die der Animator abspielt
   ├─ widgets.rs      Kasten und Listenzeile, gleich für alle Ansichten
   ├─ title_bar.rs    eigene Titelleiste (+ window_drag.rs für macOS)
   └─ backdrop.rs · log.rs · status_bar.rs · layout.rs · view.rs
```

## Erweitern

**Neues Upgrade:** in `src/game/upgrades.rs` an `UPGRADES` anhängen (nach `unlock_at` sortiert,
ein Test prüft das). Level werden per `id` gespeichert, alte Spielstände bleiben gültig. Eine neue
Effekt-Art braucht zusätzlich Text in `Effect::label`/`kind_help` (gleiche Datei) – der Compiler
erinnert daran.

```rust
up("fusion.core", 5e21, 1.15, 4e21, Effect::PerSecond(3e15), "kernfusion im taschenformat."),
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

**Neues Event:** `EventKind` in `src/game/events.rs` erweitern (`title`, `description`, Wirkung in
`action`, ggf. eine neue `BuffKind`) und in `EventKind::ALL` und `weight()` eintragen.
Hilfe-Seite und Admin-Panel übernehmen es automatisch.

**Neue Ansicht:** `Screen` in `src/ui/msg.rs` erweitern (inkl. `ALL`), `Page` implementieren und in
`ClickerApp::page`/`page_mut` eintragen – der Compiler zeigt beide Stellen.

**Neue Taste:** eine Zeile im `keymap!` in `src/ui/keys.rs` – Bindung, Statuszeile und Hilfe lesen von dort.

## Lizenz

Code: [MIT](LICENSE). Die Schrift JetBrains Mono in `assets/fonts/` steht unter der
[SIL Open Font License 1.1](assets/fonts/OFL.txt).
