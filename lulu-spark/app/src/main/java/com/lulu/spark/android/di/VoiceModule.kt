package com.lulu.spark.android.di

import com.lulu.spark.android.chat.commands.AndroidVoiceRecorder
import com.lulu.spark.android.chat.commands.VoiceRecorder
import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent

@Module
@InstallIn(SingletonComponent::class)
abstract class VoiceModule {
    @Binds
    abstract fun voiceRecorder(impl: AndroidVoiceRecorder): VoiceRecorder
}
