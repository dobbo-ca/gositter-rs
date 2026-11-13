PreEnqueuePlugin = &SchedulingGates{}
var _ fwk.EnqueueExtensions = &SchedulingGates{}

func (pl *SchedulingGates) Name() string {
	return Name
}

func (pl *SchedulingGates) PreEnqueue(ctx context.Context, p *v1.Pod) *fwk.Status {
	if len(p.Spec.SchedulingGates) == 0 {
		return nil
	}
	gates := make([]string, 0, len(p.Spec.SchedulingGates))
	for _, gate := range p.Spec.SchedulingGates {
		gates = append(gates, gate.Name)
	}
	return fwk.NewStatus(fwk.UnschedulableAndUnresolvable, fmt.Sprintf("waiting for scheduling gates: %v", gates))
}

// EventsToRegister returns the possible events that may make a Pod
// failed by this plugin schedulable.
func (pl *SchedulingGates) EventsToRegister(_ context.Context) ([]fwk.ClusterEventWithHint, error) {
	return []fwk.ClusterEventWithHint{
		// Pods can be more schedulable once it's gates are removed
		{Event: fwk.ClusterEvent{Resource: fwk.TargetPod, ActionType: fwk.UpdatePodSchedulingGatesEliminated}, QueueingHintFn: pl.isSchedulableAfterUpdateTargetPodSchedulingGatesEliminated},
	}, nil
}

// New initializes a new plugin and returns it.
func New(_ context.Context, _ runtime.Object, _ fwk.Handle, _ feature.Features) (fwk.Plugin, error) {
	return &SchedulingGates{}, nil
}

func (pl *SchedulingGates) isSchedulableAfterUpdateTargetPodSchedulingGatesEliminated(logger klog.Logger, pod *v1.Pod, oldObj, newObj interface{}) (fwk.QueueingHint, error) {
	return fwk.Queue, nil
}
