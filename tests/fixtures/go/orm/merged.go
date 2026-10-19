package orm

func Merged() {
	GetEngine().
		Join("INNER").
		Or("poster_id=?")
//^ d: none
	GetEngine().Join("INNER").Or("poster_id=?")
	//                        ^ d: none
	GetEngine().Where("x").And("y")
	//                     ^ d: orm/engine.go:17
	GetEngine().
		Where("x").
		And("y")
//^ d: orm/engine.go:17
}
