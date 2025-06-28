# Game Theme

Game theme is main theme during an active game.

`Game Theme.wav` is the final rendered version to be played as the game starts.
`Game Theme Battle {x}.wav` are short tracks that indicate a battle that can be played on be `x`. I.e., `Game Theme Battle 1.wav` should be played over the main theme, and its audio starts on beat 1, so it can be played at the start of any bar. `Game Theme Battle 2.wav`'s audio starts on beat 2, and there is a beat of silence in the audio, so if a battle occurs just before beat 2, you should play the battle file, starting 1 beat in. The time signature is `3/4` at `108` bpm, meaning each beat takes `1/(108/60) = 5/9 seconds ~= 0.55556ms`.
`Game Theme Example.wav` demonstrates how battles should sound when incorporated into the main theme.
