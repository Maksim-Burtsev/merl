class_name Actor
extends Node2D

var direction := Vector2.ZERO

func die() -> void:
	queue_free()

func step(delta: float) -> void:
	position += direction * delta
#            ^ d: actors/actor.gd:4
#                        ^ d: actors/actor.gd:9
#                        status: delta: local
