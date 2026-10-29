"""PlayerAggregateImpl: the implementation of ``PlayerAggregate``, written
once by ``angzarr scaffold python`` and yours from then on: ``--missing``
adds the methods it lacks, each a TODO.
"""

from __future__ import annotations

from angzarr_player._gen.io.angzarr.examples.v1.player_pb2 import (
    DecideMove,
    MoveDecided,
    PlayerState,
)
from angzarr_player.player_aggregate import PlayerAggregate


class PlayerAggregateImpl(PlayerAggregate):
    def decide_move(self, cmd: DecideMove, state: PlayerState) -> MoveDecided:
        raise NotImplementedError  # TODO: decide DecideMove

    def apply_move_decided(self, event: MoveDecided, state: PlayerState) -> None:
        raise NotImplementedError  # TODO: fold MoveDecided into the state
