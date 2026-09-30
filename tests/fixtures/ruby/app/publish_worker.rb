class PublishWorker
  def perform(id)
    scheduled_status = ScheduledStatus.find(id)
    scheduled_status.destroy!
  end
end
