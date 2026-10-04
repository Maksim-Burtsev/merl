extends "res://actors/actor.gd"
#              ^ d: actors/actor.gd:1

func chase(target: Player) -> void:
	direction = (target.position - position).normalized()
#^ d: actors/actor.gd:4
#             ^ d: actors/enemy.gd:4
