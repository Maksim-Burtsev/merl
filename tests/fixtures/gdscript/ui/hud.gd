extends CanvasLayer

func show_coins(total: int) -> void:
	%HealthBar.value = total
# ^ d: none
	$Coins/Label.text = str(total)
# ^ d: none
#       ^ d: none
	GameState.coins_changed.connect(show_coins)
#^ d: globals/game_state.gd:1
#          ^ d: globals/game_state.gd:3
#          status: via GameState
#                                ^ d: ui/hud.gd:3
