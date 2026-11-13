package com.discordclone.domain.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class User(
    @SerialName("id") val id: String,
    @SerialName("username") val username: String,
    @SerialName("email") val email: String,
    @SerialName("bio") val bio: String? = null,
    @SerialName("avatar_url") val avatarUrl: String? = null,
    @SerialName("banner_url") val bannerUrl: String? = null,
    @SerialNamatus") val status: UserStatus = UserStatus.OFFLINE,
    @SerialName("custom_status") val customStatus: String? = null,
    @SerialName("created_at") val createdAt: String
)

@Serializable
enum class UserStatus {
    @SerialName("online") ONLINE,
    @SerialName("idle") IDLE,
    @SerialName("dnd") DND,
    @SerialName("offline") OFFLINE
}
