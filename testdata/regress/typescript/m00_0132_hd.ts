 = new TestTelemetryService();

		for (const [fromProvider, toProvider] of [
			[undefined, 'openai'],
			['vscode-proxy', undefined],
			[undefined, undefined],
			['', 'vscode-proxy'],
			['openai', ''],
			['custom-provider', 'vscode-proxy'],
			['openai', 'https://private-provider.example/api'],
			['openai', 'openai'],
			['vscode-proxy', 'vscode-proxy'],
		]) {
			reportCodexProviderSwitch(telemetryService, fromProvider, toProvider, true);
		}

		assert.deepStrictEqual(telemetryService.events, []);
	});
});
