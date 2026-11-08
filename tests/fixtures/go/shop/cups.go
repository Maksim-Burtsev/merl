package shop

type Saucer struct {
	Cup int
}

func Pour() {
	cupa := struct{ Cup int }{}
	_ = cupa.Cup
	//       ^ d: shop/cups.go:8
	var cupb struct {
		Cup int
	}
	_ = cupb.Cup
	//       ^ d: shop/cups.go:12
	cupc := &struct {
		Cup int
	}{Cup: 1}
	_ = cupc.Cup
	//       ^ d: shop/cups.go:17
}
