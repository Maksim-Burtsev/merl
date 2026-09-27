// Command worker drains the order jobs the API queues: shipping and syncs.
package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"time"

	"example.com/orders/worker/internal/queue"
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt)
	defer stop()

	q := queue.New()
	q.Push(queue.Job{Kind: "sync"})
	if err := run(ctx, q); err != nil {
		log.Fatal(err)
	}
}

func run(ctx context.Context, q queue.Queue) error {
	for {
		select {
		case <-ctx.Done():
			return drain(q)
		case <-time.After(time.Second):
		}
		job, ok := q.Pop()
		if !ok {
			continue
		}
		if err := handle(job); err != nil {
			return fmt.Errorf("job for order %d: %w", job.OrderID, err)
		}
	}
}

// drain runs the jobs left in q before the worker stops.
func drain(q queue.Queue) error {
	jobs := make(
		[]queue.Job, 0, 16,
	)
	for job, ok := q.Pop(); ok; job, ok = q.Pop() {
		jobs = append(jobs, job)
	}
	for _, job := range jobs {
		if err := handle(job); err != nil {
			return err
		}
	}
	return nil
}

func handle(job queue.Job) error {
	switch job.Kind {
	case "ship", "sync":
		log.Printf("order %d → %s", job.OrderID, job.Kind)
		return nil
	default:
		return fmt.Errorf("unknown job kind %q", job.Kind)
	}
}
