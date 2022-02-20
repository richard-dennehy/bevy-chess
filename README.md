# Bevy Chess

Simple chess game using [the Bevy game engine](https://bevyengine.org/), initially based on [this blog post](https://caballerocoll.com/blog/bevy-chess-tutorial/), but further developed to support special moves, show valid moves during each turn, and using custom assets.

This implementation supports en passant, pawn two-step moves, castling, and pawn promotion.

The game properly detects check, checkmate, and stalemate, but does not recognise other draw conditions (e.g. threefold repetition).