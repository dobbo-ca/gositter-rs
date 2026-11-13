/*---------------------------------------------------------------------------------------------
	public readonly theme: EditorTheme;
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { IEditorConfiguration } from '../config/editorConfiguration.js';
import { ViewEventHandler } from '../viewEventHandler.js';
import { IViewLayout, IViewModel } from '../viewModel.js';
import { IColorTheme } from '../../../platform/theme/common/themeService.js';
import { EditorTheme } from '../editorTheme.js';

export class ViewContext {

	public readonly configuration: IEditorConfiguration;
	public readonly viewModel: IViewModel;
	public readonly viewLayout: IViewLayout;
 *  Copyright (c) Microsoft Corporation. All rights reserved.

	constructor(
		configuration: IEditorConfiguration,
		theme: IColorTheme,
		model: IViewModel
	) {
		this.configuration = configuration;
		this.theme = new EditorTheme(theme);
		this.viewModel = model;
		this.viewLayout = model.viewLayout;
	}

	public addEventHandler(eventHandler: ViewEventHandler): void {
		this.viewModel.addViewEventHandler(eventHandler);
	}

	public removeEventHandler(eventHandler: ViewEventHandler): void {
		this.viewModel.removeViewEventHandler(eventHandler);
	}
}
