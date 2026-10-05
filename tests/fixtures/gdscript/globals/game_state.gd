extends Node

signal coins_changed(total: int)
signal died

var coins := 0

func add_coins(amount: int) -> void:
	coins += amount
#^ d: globals/game_state.gd:6
#         ^ d: globals/game_state.gd:8
#         status: amount: local
	coins_changed.emit(coins)
#^ d: globals/game_state.gd:3
#              ^ d: none
#                   ^ d: globals/game_state.gd:6

func reset() -> void:
	emit_signal("coins_changed")
#             ^ d: globals/game_state.gd:3
	died.emit()
#^ d: globals/game_state.gd:4
