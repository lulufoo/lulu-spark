package com.lulu.workbench.android.di

import android.content.Context
import com.lulu.workbench.android.WorkbenchApp
import com.lulu.workbench.android.agent.facade.WorkbenchRuntime
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object RuntimeModule {
    @Provides
    @Singleton
    fun runtime(@ApplicationContext ctx: Context): WorkbenchRuntime =
        (ctx as WorkbenchApp).runtime
}
