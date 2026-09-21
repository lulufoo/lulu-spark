package com.lulu.workbench.android.di

import com.lulu.workbench.android.chat.commands.AndroidVoiceRecorder
import com.lulu.workbench.android.chat.commands.VoiceRecorder
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
