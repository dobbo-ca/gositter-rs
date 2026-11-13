// Copyright 2013-2015 Serilog Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

namespace Serilog.Configuration;

/// <summary>
/// Controls filter configuration.
/// </summary>
public class LoggerFilterConfiguration
{
    readonly LoggerConfiguration _loggerConfiguration;
    readonly Action<ILogEventFilter> _addFilter;

    internal LoggerFilterConfiguration(
        LoggerConfiguration loggerConfiguration,
        Action<ILogEventFilter> addFilter)
    {
        _loggerConfiguration = Guard.AgainstNull(loggerConfiguration);
        _addFilter = Guard.AgainstNull(addFilter);
    }

    /// <summary>
    /// Filter out log events from the stream based on the provided filter.
    /// </summary