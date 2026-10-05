package com.lulu.spark.android.di

import android.content.Context
import com.lulu.spark.android.SparkApp
import com.lulu.spark.android.agent.facade.SparkRuntime
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
    fun runtime(@ApplicationContext ctx: Context): SparkRuntime =
        (ctx as SparkApp).runtime
}
