package queue

import "encoding/json"

// listClient is the part of a Redis client the queue uses.
type listClient interface {
	LPush(key string, value []byte) error
	RPop(key string) ([]byte, error)
}

// redisQueue keeps the jobs in a Redis list that every worker replica shares.
type redisQueue struct {
	client listClient
	key    string
}

// NewRedis returns a queue over the Redis list at key.
func NewRedis(client listClient, key string) Queue {
	return &redisQueue{client: client, key: key}
}

func (q *redisQueue) Push(job Job) {
	b, _ := json.Marshal(job)
	_ = q.client.LPush(q.key, b)
}

func (q *redisQueue) Pop() (Job, bool) {
	b, err := q.client.RPop(q.key)
	if err != nil || b == nil {
		return Job{}, false
	}
	var job Job
	if json.Unmarshal(b, &job) != nil {
		return Job{}, false
	}
	return job, true
}
