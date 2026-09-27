// Package queue holds the jobs the worker has yet to run.
package queue

import "sync"

// Job is one unit of work for an order.
type Job struct {
	Kind    string
	OrderID int64
}

// Queue is what the worker takes its jobs from.
type Queue interface {
	Push(job Job)
	Pop() (Job, bool)
}

type memQueue struct {
	mu   sync.Mutex
	jobs []Job
}

// New returns a queue in memory, for local runs and tests.
func New() *memQueue {
	return &memQueue{}
}

func (q *memQueue) Push(job Job) {
	q.mu.Lock()
	defer q.mu.Unlock()
	q.jobs = append(q.jobs, job)
}

func (q *memQueue) Pop() (Job, bool) {
	q.mu.Lock()
	defer q.mu.Unlock()
	if len(q.jobs) == 0 {
		return Job{}, false
	}
	job := q.jobs[0]
	q.jobs = q.jobs[1:]
	return job, true
}
