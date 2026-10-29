"""PlayerAggregate: the interface of ``io.angzarr.examples.v1.PlayerState``,
written once by ``angzarr scaffold python`` and yours from then on:
``--missing`` adds the methods it lacks. It names only the domain's messages;
the generated dispatch (``_gen/.../player_aggregate_angzarr.py``) calls it.
"""

from __future__ import annotations

from typing import Protocol

from angzarr_player._gen.io.angzarr.examples.v1.player_pb2 import (
    DecideMove,
    MoveDecided,
    PlayerState,
)


class PlayerAggregate(Protocol):
    def decide_move(self, cmd: DecideMove, state: PlayerState) -> MoveDecided: ...

    def apply_move_decided(self, event: MoveDecided, state: PlayerState) -> None: ...
