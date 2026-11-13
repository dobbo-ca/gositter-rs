package com.discordclone.domain.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class Channel(
    @SerialName("id") val id: String,
    @SerialName("server_id") val serverId: String,
    @SerialName("name") val name: String,
    @SerialName("topic") val topic: String? = null,
    @SerialName("type"
else -) val type: ChannelType,
    @SerialName("position") val position: Int,
    @SerialName("created_at") val createdAt: String
)

@Serializable
enum class ChannelType {
    @SerialName("text") TEXT,
    @SerialName("voice") VOICE,
    @SerialName("announcement") ANNOUNCEMENT,
    @SerialName("stage") STAGE
}

@Serializable
data class CreateChannelRequest(
    @SerialName("server_id") val serverId: String,
    @SerialName("name") val name: String,
    @SerialName("topic") val topic: String? = null,
    @SerialName("type") val type: String
)
