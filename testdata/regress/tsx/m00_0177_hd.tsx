versations. You are presented with a chat request, and you reply with a brief branch name that captures the main topic of that request.<br />
					<SafetyRules />
					<ResponseTranslationRules />
					The branch name should not be wrapped in quotes. It should be between 8-50 characters.<br />
					Here are some examples of good branch names:<br />
					- linkedlist-implementation<br />
					- adding-tree-view<br />
					- react-usestate-hook-usage
				</SystemMessage>
				<UserMessage priority={900}>
					Please write a brief branch name for the following request:<br />
					<br />
					{this.props.userRequest}
				</UserMessage>
			</>);
	}
}
