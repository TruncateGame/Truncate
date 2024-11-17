# QA Checklist

An informal list of micro (and not-so-micro) interactions to check when doing large updates, such as upgrading egui.

## Dragging

### Dragging from the hand
- [ ] Dragging a tile on desktop should place it under the cursor with the same offset it was clicked at
- [ ] Dragging a tile on touchscreen should animate it up above the drag point so as to not obscure it
- [ ] Dragging a tile over the board should snap it to any playable squares, but not land or other tiles
- [ ] Snapping the tile to square should animate both its size and position
- [ ] When dragging there should be no deadzone between squares on the board, the tile should snap from one to the other
- [ ] Dragged tiles should preview above all art, especially destroyed tiles
- [ ] Dragging a selected tile in the hand (with the highlight) should un-select it
- [ ] Dragging a tile from the hand should unselect any tiles on the board
- [ ] With the dictionary overlay open, dragging a hand tile should close the overlay

### Dragging from the board
- [ ] Dragging a tile on the board with any highlight ring shouldn't show the highlight on the dragged tile
- [ ] Dragging a tile from the board should not drag the grass overlay
- [ ] Starting a drag on a board tile should highlight it gold but keep it snapped to its position
- [ ] Dragging a board tile over another should snap, highlight both gold, and swap their glyphs
- [ ] Dragging a board tile over empty squares should not do any snapping, and the source tile should be highlighted gold
- [ ] Dragging a board tile over the opponents tiles should not preview the swap

## Clicking

### Clicking from the hand
- [ ] Hovering a tile in the hand should show a highlight ring
- [ ] Clicking a hand tile should select it
- [ ] Clicking it again should de-select
- [ ] Clicking a different tile should change selection
- [ ] With a tile selected, hovering a board square should show thew tile preview
- [ ] With a tile selected, the preview hover should appear above board decorations, especially destroyed tiles
- [ ] With a tile selected, the preview hover should not have the grass overlay applied
- [ ] With a tile selected, hovering an occupied board square should do nothing
- [ ] With the dictionary overlay open, selecting a hand tile should close the overlay

### Clicking from the board
- [ ] Hovering a tile on the board should should a highlight ring
- [ ] Clicking a tile on the board should select it
- [ ] Clicking a selected tile on the board should unselect it
- [ ] With a tile selected, clicking an empty square should deselect it
- [ ] With a tile selected, hovering another board tile should show a preview of the swap, with grass texture overlaid
- [ ] Hovering existing tiles on the board should have a deadzone between tiles where nothing is selected
- [ ] With a _hand_ tile selected, hovering on the board should have no deadzone and should snap from square to square

### Other clicks
- [ ] With the actions menu open, clicking the button again should close it
- [ ] With the actions menu open, clicking anything on the board should close it
- [ ] With the actions menu open, clicking anything in the battles should close it
- [ ] With the actions menu open, clicking anything in the hand should close it
- [ ] With the actions menu open, clicking mute/unmute should _not_ close it
- [ ] With the actions menu open, clicking other buttons should close it where logical

## Areas
- [ ] The timer region should span the full width of the screen, and be translucent when the board sits underneath
- [ ] The hand region should span the full width of the screen, and be translucent when the board sits underneath
- [ ] When open, the battle region should span the full width of the screen, and be translucent when the board sits underneath
- [ ] When open, the battle region, timer bar, and hand regions should look seamless w.r.t. their overlays joining together
- [ ] Opening the actions menu should fade the board slightly

## Dictionary
- [ ] Opening the dictionary should disable board panning/zooming
- [ ] Opening the dictionary should fade out the board, but not the tiles
- [ ] Focusing the dictionary input should not fade out the board
- [ ] Typing in the dictionary input _should_ fade out the board and show the definition
- [ ] Pressing `Esc` should close the dictionary

## Keyboard controls

## Board panning & zooming

## Board editor

## Animations
- [ ] Battle animation
- [ ] Wind animations

## Game winning UI

## Timers
- [ ] When not your turn, tiles should be grey
- [ ] Without time enabled, timers at the top should have full bars and specify "Your turn!" and "Playing" for your and the computer's turns respectively
- [ ] Timers at the top should be coloured to the player when it is their turn, and faded when not

## Lobbies

## Modals

### Daily puzzle modal

### Ad hoc puzzle modal
