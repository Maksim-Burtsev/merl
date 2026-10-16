package twin

type Kettle struct {
	Spout int
}

func (k Kettle) Pour() int { return k.Spout }

func Whistle() int { return 2 }

func Mug() int { return 3 }
