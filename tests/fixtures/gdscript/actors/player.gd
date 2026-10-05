class_name Player extends CharacterBody2D
#                         ^ d: none

const SPEED := 200.0
const COIN = preload("res://actors/coin.tscn")
#                           ^ d: actors/coin.tscn:1

@export var health := 100
@onready var sprite = $Sprite2D
#                      ^ d: none
static var count = 0

enum State { IDLE, RUN, JUMP }
enum {
	LEFT,
	RIGHT = 2,
}

var state := State.IDLE
#            ^ d: actors/player.gd:13
#                  ^ d: actors/player.gd:13

class Stats extends RefCounted:
	var speed: float = 200.0
	static func create() -> Stats:
		return Stats.new()
#        ^ d: actors/player.gd:23

func _physics_process(delta: float) -> void:
	var direction := Input.get_axis("left", "right")
	velocity.x = direction * SPEED
#             ^ d: actors/player.gd:30
#             status: _physics_process.direction (local)
#                         ^ d: actors/player.gd:4
	move_and_slide()
	for coin in get_tree().get_nodes_in_group("coins"):
		if coin.position.distance_to(position) < 8.0:
#    ^ d: actors/player.gd:36
			pick_up(1)
#  ^ d: actors/player.gd:49
	match state:
		State.IDLE:
#       ^ d: actors/player.gd:13
			sprite.play("idle")
#  ^ d: actors/player.gd:9
		LEFT:
# ^ d: actors/player.gd:15

func pick_up(value: int) -> void:
	GameState.add_coins(value)
#          ^ d: globals/game_state.gd:8
#          status: add_coins: via GameState
#^ d: globals/game_state.gd:1
#                    ^ d: actors/player.gd:49
	take_damage(0)
#^ d: actors/player.gd:67
	count += 1
#^ d: actors/player.gd:11
	var bag = {"speed": 1}
#            ^ d: none
	var make = Stats.create()
#                 ^ d: actors/player.gd:25
	var hit = func(amount): return amount * RIGHT
#                               ^ d: actors/player.gd:63
#                                        ^ d: actors/player.gd:16

func take_damage(amount: int) -> void:
	health -= amount
#^ d: actors/player.gd:8
	super()
	"""
func take_damage(amount):
var health := 0
	"""

func _ready() -> void:
	var target: Player = null
#            ^ d: actors/player.gd:1
	var other: Actor = null
#           ^ d: actors/actor.gd:1
	target.die()
#       ^ d: actors/actor.gd:6
