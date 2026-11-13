es;
        queryProperties.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2_KHR;
        queryProperties.pNext = queryDeviceProperties;
        vkGetPhysicalDeviceProperties2KHR = (PFN_vkGetPhysicalDeviceProperties2KHR)vkGetInstanceProcAddr(kInstance, "vkGetPhysicalDeviceProperties2KHR");
        vkGetPhysicalDeviceProperties2KHR(kPhysicalDevice, &queryProperties);

        if (VK_VERSION_MAJOR(instanceApiVersion) >= 1 && VK_VERSION_MINOR(instanceApiVersion) >= 1)
        {
            info.subgroupSize = physicalDeviceSubgroupProperties.subgroupSize;
            if (physicalDeviceSubgroupProperties.supportedStages & VK_SHADER_STAGE_COMPUTE_BIT)
            {
                info.supportSubgroupBasic = physicalDeviceSubgroupProperties.supportedOperations & VK_SUBGROUP_FEATURE_BASIC_BIT;
                info.supportSubgroupVote = physicalDeviceSubgroupProperties.supportedOperations & VK_SUBGROUP_FEATURE_VOTE_BIT;
                info.supportSubgroupBallot = physicalDeviceSubgroupProperties.supportedOperations & VK_SUBGROUP_FEATURE_BALLOT_BIT;
                info.supportSubgroupShuffle = physicalDeviceSubgroupProperties.supportedOperations & VK_SUBGROUP_FEATURE_SHUFFLE_BIT;
            }