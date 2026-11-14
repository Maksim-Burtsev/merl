package rival

func Nibble(o any) int {
	nibble := 0
	switch nibble := o.(type) {
	case int:
		return nibble
		//     ^ d: rival/nibble.go:5
	}
	return nibble
	//     ^ d: rival/nibble.go:4
}
