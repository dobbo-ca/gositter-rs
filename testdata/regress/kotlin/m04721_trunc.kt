/*
 * Copyright (c) HashiCorp, Inc.
 * SPDX-License-Identifier: MPL-2.0
 */

package tests

import AzureRM
import AzureRMBetaVersion
import org.junit.Assert.assertTrue
import org.junit.Test
import useTeamCityGoTest

class ConfigurationTests {
    @Test
    fun buildShouldFailOnError() {
        val project = AzureRM("public", TestConfiguration())
        project.buildTypes.forEach { bt ->
            assertTrue("Build '${bt.id}' should fail on errors!", bt.failureConditions.errorMessage)
        }
    }

    @Test
    fun buildShouldHaveGoTestFeature() {
        val project = AzureRM("public", TestConfiguration())
        project.buildTypes.forEach{ bt ->
            var exists = false
  