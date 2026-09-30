class Scheduler
  def publish
    due.find_each do |scheduled_status|
      PublishWorker.perform_at(scheduled_status.scheduled_at)
      #                        ^ d: app/scheduler.rb:3
    end
  end
end
