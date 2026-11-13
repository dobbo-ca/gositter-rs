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
        Index(value = ["channelId", "id"])
    ]
)
data class MessageEntity(
    @PrimaryKey
    val id: String,
    val channelId: String,
    val content: String,
    val authorId: String,
    val authorUsername: String,
    val authorDiscriminator: String?,
    val authorAvatarUrl: String?,
    val createdAt: Long,
    val editedAt: Long?,
    val isPending: Boolean = false, // For offline messages
    val lastSynce