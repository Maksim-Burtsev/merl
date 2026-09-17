package main

import "fmt"

type Job interface {
	Run(id int)
}

type ImportJob struct{}

func (i *ImportJob) Run(id int) {
	fmt.Println(id)
}

type ExportJob struct{}

func (e ExportJob) Run(id int) {
	fmt.Println(id)
}

type Batch struct{}

func (b Batch) Run(id int, retries int) {
	fmt.Println(id, retries)
}

type LoudNotifier struct{}

func (l *LoudNotifier) Send(text string) {
	fmt.Println(text)
}

type Sweeper interface {
	Sweep()
}

type NightlySweeper struct{}

func (n NightlySweeper) Sweep() {
}
