# hololive OCG Deck Converter

Convert your hOCG deck between various formats.

https://qrimpuff.github.io/hocg-deck-convert/

<p></p>
Formats supported:

- [Deck Log (Bushiroad)](https://decklog-en.bushiroad.com/)
- [holoDelta](https://holodelta.azurewebsites.net/)
- [holoBuri](https://holoburi.com/)
- [Tabletop Simulator (by Noodlebrain)](https://steamcommunity.com/sharedfiles/filedetails/?id=3302530285)
- Proxy sheets (PDF)
- Price check (with [Yuyu-tei](https://yuyu-tei.jp/top/hocg) or [TGGPlayer](https://www.tcgplayer.com/categories/trading-and-collectible-card-games/hololive-official-card-game))
- Card list (CSV), for decks and collections

### Card list (CSV)

Export a deck or your collection as a spreadsheet, one row per printing (opens in Excel and Google
Sheets). Choose the columns and the language of the names on the export page, so you can read
English names for Japanese cards.

Import it back to get the same cards, or import your own list: a `Card Number` and a `Quantity`
column are enough (add `Rarity` or `Set Code` to pick a printing). A `Deck` column (`Oshi`,
`Main Deck`, `Cheer Deck`) imports a deck. Rows that can't be matched exactly are listed after the import.

![hololive OCG Deck Converter](https://qrimpuff.github.io/hocg-deck-convert/assets/image.png)

## Development

Run the following command in the root of the project to start the Dioxus dev server:

```bash
dx serve --release
```

- Open the browser to http://localhost:8080/hocg-deck-convert