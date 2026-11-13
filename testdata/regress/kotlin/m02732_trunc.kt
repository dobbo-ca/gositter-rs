package com.discordclone.data.local.entity

import androidx.room.Entity
import androidx.room.PrimaryKey
import androidx.room.Index
import com.discordclone.domain.model.Message
import com.discordclone.domain.model.User

/**
 * Room entity for local message caching
 * 7-day retention policy applied via cleanup job
 */
@Entity(
    tableName = "messages",
    indices = [
        Index(value = ["channelId", "createdAt"]),
       