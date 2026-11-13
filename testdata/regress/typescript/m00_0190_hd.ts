event;

	constructor(
		@ILogService private readonly logService: ILogService
	) {
		super();
	}

	fireOnWillShutdown(): void {
		this.logService.trace('Lifecycle#onWillShutdown.fire()');

		this._onWillShutdown.fire();
	}
}
