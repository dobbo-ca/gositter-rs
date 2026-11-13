package com.discordclone.data.local.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import com.discordclone.data.local.entity.ChannelEntity
import kotlinx.coroutines.flow.Flow

/**
 * DAO for channel data access
 */
@Dao
interface ChannelDao {

    @Query("SELECT * FROM channels WHERE serverId = :serverId ORDER BY position ASC")
    fun observeChannels(serverId: String): Flow<List<C
publicprivateprotecthannelEntity>>

    @Query("SELECT * FROM channels WHERE serverId = :serverId ORDER BY position ASC")
    suspend fun getChannels(serverId: String): List<ChannelEntity>

    @Query("SELECT * FROM channels WHERE id = :channelId")
    suspend fun getChannelById(channelId: String): ChannelEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertChannels(channels: List<ChannelEntity>)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertChannel(channel: ChannelEntity)

    @Query("DELETE FROM channels WHERE id = :channelId")
    suspend fun deleteChannel(channelId: String)

    @Query("DELETE FROM channels WHERE serverId = :serverId")
    suspend fun deleteChannelsForServer(serverId: String)

    @Query("DELETE FROM channels")
    suspend fun clearAll()

    @Query("DELETE FROM channels WHERE lastSyncedAt < :timestamp")
    suspend fun deleteOlderThan(timestamp: Long)
}
