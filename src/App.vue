<template>
  <div id="app">
    <!-- Header -->
    <header class="app-header">
      <div class="header-content">
        <h1>🔬 Clustering Tool for LAMMPS</h1>
        <p>Análisis avanzado de clusters en simulaciones de Dinámica Molecular</p>
        <div class="header-info">
          <span class="info-badge" :class="{ 'badge-success': dependenciesOk, 'badge-error': dependenciesChecked && !dependenciesOk }">
            {{ pythonStatus }}
          </span>
        </div>
      </div>
    </header>

    <main class="app-main">
      <!-- Alert de dependencias faltantes -->
      <div v-if="dependenciesChecked && !dependenciesOk" class="alert alert-error">
        <h3>⚠️ Dependencias Python faltantes</h3>
        <p>Por favor instala las siguientes librerías:</p>
        <ul>
          <li v-for="dep in missingDependencies" :key="dep">
            <code>pip install {{ dep }}</code>
          </li>
        </ul>
        <button @click="checkDependencies" class="btn-secondary">
          🔄 Verificar nuevamente
        </button>
      </div>

      <!-- Panel de Configuración -->
      <div class="config-panel card">
        <div class="card-header">
          <h2>⚙️ Configuración del Clustering</h2>
          <div class="header-actions">
            <button @click="loadSavedConfig" class="btn-icon" title="Cargar última configuración">
              📂
            </button>
            <button @click="saveCurrentConfig" class="btn-icon" title="Guardar configuración">
              💾
            </button>
          </div>
        </div>
        
        <div class="form-grid">
          <!-- Archivo de Entrada -->
          <div class="form-group span-2">
            <label for="input-file">
              📁 Archivo Dump de Entrada
              <span class="required">*</span>
            </label>
<div class="file-input-group">
  <input 
    type="text" 
    id="input-file"
    v-model="params.input_file"
    placeholder="Ruta al archivo .dump"
    class="file-input"
    :class="{ 'input-error': validationErrors.input_file }"
  >
  <button @click="selectFile" class="btn-secondary">📂 Seleccionar dump</button>
  <button @click="leerArchivoDump" class="btn-secondary">🔍 Leer dump</button>
</div>

            <span v-if="validationErrors.input_file" class="error-text">
              {{ validationErrors.input_file }}
            </span>
          </div>

          <!-- Directorio de Salida -->
          <div class="form-group span-2">
            <label for="output-dir">
              📂 Directorio de Salida
            </label>
            <input 
              type="text" 
              id="output-dir"
              v-model="params.output_dir"
              placeholder="clusters_output"
              class="file-input"
            >
          </div>

          <!-- Parámetros Principales -->
          <div class="form-group">
            <label for="min-atoms">
              🔢 Mínimo de Átomos por Cluster
            </label>
            <input 
              type="number" 
              id="min-atoms"
              v-model.number="params.min_atoms"
              min="1"
              class="param-input"
              :class="{ 'input-error': validationErrors.min_atoms }"
            >
          </div>

          <div class="form-group">
            <label for="max-iterations">
              🔁 Máximo de Iteraciones
            </label>
            <input 
              type="number" 
              id="max-iterations"
              v-model.number="params.max_iterations"
              min="1"
              max="20"
              class="param-input"
            >
          </div>

          <!-- Umbrales de Métricas -->
          <div class="form-group">
            <label for="silhouette-threshold">
              📊 Umbral Silhouette: <strong>{{ params.silhouette_threshold.toFixed(2) }}</strong>
            </label>
            <input 
              type="range" 
              id="silhouette-threshold"
              v-model.number="params.silhouette_threshold"
              min="0"
              max="1"
              step="0.05"
              class="param-slider"
            >
          </div>

          <div class="form-group">
            <label for="db-threshold">
              📉 Umbral Davies-Bouldin: <strong>{{ params.davies_bouldin_threshold.toFixed(1) }}</strong>
            </label>
            <input 
              type="range" 
              id="db-threshold"
              v-model.number="params.davies_bouldin_threshold"
              min="0"
              max="5"
              step="0.1"
              class="param-slider"
            >
          </div>

          <!-- Opciones Avanzadas -->
          <div class="form-group">
            <label for="clusters-per-level">
              🎯 Clusters por Nivel
            </label>
            <select 
              id="clusters-per-level"
              v-model="params.n_clusters_per_level"
              class="param-select"
            >
              <option :value="null">Automático (Mean Shift)</option>
              <option :value="2">2 (KMeans)</option>
              <option :value="3">3 (KMeans)</option>
              <option :value="4">4 (KMeans)</option>
              <option :value="5">5 (KMeans)</option>
            </select>
          </div>

          <div class="form-group">
            <label for="quantile">
              🎲 Quantile: <strong>{{ params.quantile.toFixed(2) }}</strong>
            </label>
            <input 
              type="range" 
              id="quantile"
              v-model.number="params.quantile"
              min="0.1"
              max="0.5"
              step="0.05"
              class="param-slider"
            >
          </div>
        </div>

        <!-- Botones de Acción -->
        <div class="action-buttons">
          <button 
            @click="runClustering" 
            :disabled="isProcessing || !dependenciesOk || !isFormValid"
            class="btn-primary"
            type="button"
          >
            <span v-if="isProcessing">
              <span class="spinner"></span>
              Procesando...
            </span>
            <span v-else>
              🚀 Ejecutar Clustering
            </span>
          </button>
          
          <button 
            v-if="isProcessing"
            @click="cancelProcess"
            class="btn-danger"
            type="button"
          >
            ❌ Cancelar
          </button>
          
          <button 
            @click="resetParams"
            :disabled="isProcessing"
            class="btn-secondary"
            type="button"
          >
            🔄 Restablecer
          </button>
        </div>
      </div>

      <!-- Panel de Progreso -->
      <div v-if="isProcessing || progressLogs.length > 0" class="progress-panel card">
        <div class="card-header">
          <h2>📊 Progreso del Clustering</h2>
          <button @click="clearLogs" class="btn-icon" title="Limpiar logs">
            🗑️
          </button>
        </div>
        
        <div class="progress-bar-container">
          <div class="progress-bar" :style="{ width: progressPercentage + '%' }">
            <span class="progress-text">{{ progressPercentage.toFixed(0) }}%</span>
          </div>
        </div>

        <div ref="logsContainer" class="logs-container">
          <div 
            v-for="(log, index) in progressLogs" 
            :key="index" 
            class="log-line"
            :class="getLogClass(log)"
          >
            <span class="log-time">{{ log.time }}</span>
            <span class="log-message">{{ log.message }}</span>
          </div>
        </div>
      </div>

      <!-- Panel de Resultados -->
      <div v-if="results && results.success" class="results-panel card">
        <div class="card-header">
          <h2>✅ Resultados del Clustering</h2>
          <button 
            @click="openResultsFolder" 
            class="btn-secondary"
            title="Abrir carpeta de resultados"
          >
            📁 Abrir Carpeta
          </button>
        </div>
        
        <div class="results-summary">
          <div class="stat-card">
            <h3>Clusters Encontrados</h3>
            <p class="stat-value">{{ results.clusters_found }}</p>
          </div>
          
          <div class="stat-card">
            <h3>Átomos Totales</h3>
            <p class="stat-value">{{ results.total_atoms.toLocaleString() }}</p>
          </div>
          
          <div class="stat-card">
            <h3>Promedio por Cluster</h3>
            <p class="stat-value">{{ averageAtomsPerCluster }}</p>
          </div>
        </div>

        <!-- Lista de Clusters -->
        <div v-if="results.clusters_info && results.clusters_info.length > 0" class="clusters-section">
          <h3>📋 Detalles de Clusters ({{ results.clusters_info.length }})</h3>

          <div class="clusters-list">
            <div 
              class="cluster-item" 
              v-for="cluster in results.clusters_info" 
              :key="cluster.name"
            >
              <div class="cluster-header">
                <span class="cluster-name">{{ cluster.name }}</span>
                <span class="cluster-atoms">
                  {{ cluster.atoms.toLocaleString() }} átomos
                </span>
              </div>
              <div class="cluster-details">
                <span class="cluster-level">
                  📊 Nivel {{ cluster.level }}
                </span>
                <span class="cluster-reason">
                  {{ cluster.reason }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Mensajes de Error -->
      <div v-if="error" class="alert alert-error">
        <div class="alert-header">
          <h3>❌ Error</h3>
          <button @click="error = null" class="btn-close">×</button>
        </div>
        <pre class="error-content">{{ error }}</pre>
      </div>

      <!-- Mensajes de Éxito -->
      <div v-if="successMessage" class="alert alert-success">
        <div class="alert-header">
          <h3>✅ {{ successMessage }}</h3>
          <button @click="successMessage = null" class="btn-close">×</button>
        </div>
      </div>
    </main>

    <footer class="app-footer">
      <p>Clustering Tool v1.0.0 - Desarrollado para análisis de simulaciones LAMMPS</p>
    </footer>
  </div>
</template>

<script>
// IMPORTS CORRECTOS PARA TAURI 2
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { listen } from '@tauri-apps/api/event'
import { readTextFile } from "@tauri-apps/plugin-fs"
export default {
  name: 'App',
  data() {
    return {
      // Estado de dependencias
      dependenciesChecked: false,
      dependenciesOk: false,
      pythonVersion: '',
      missingDependencies: [],
      
      // Estado del proceso
      isProcessing: false,
      error: null,
      successMessage: null,
      results: null,
      
      // Progreso
      progressLogs: [],
      progressPercentage: 0,
      
      // Parámetros del clustering
      params: {
        input_file: '',
        output_dir: 'clusters_output',
        min_atoms: 50,
        max_iterations: 5,
        n_clusters_per_level: null,
        silhouette_threshold: 0.3,
        davies_bouldin_threshold: 1.5,
        dispersion_threshold: null,
        quantile: 0.2
      },
      
      // Validación
      validationErrors: {},
      
      // Listener de eventos
      unlistenProgress: null
    }
  },
  computed: {
    pythonStatus() {
      if (!this.dependenciesChecked) {
        return '⏳ Verificando Python...'
      }
      if (this.dependenciesOk) {
        return `✅ ${this.pythonVersion}`
      }
      return '❌ Dependencias faltantes'
    },
    
    isFormValid() {
      return this.params.input_file && 
             this.params.min_atoms > 0 && 
             this.params.max_iterations > 0 &&
             this.params.max_iterations <= 20
    },
    
    averageAtomsPerCluster() {
      if (!this.results || !this.results.clusters_found) return 0
      return Math.round(this.results.total_atoms / this.results.clusters_found).toLocaleString()
    }
  },
  methods: {
    async checkDependencies() {
      try {
        this.dependenciesChecked = false
        const result = await invoke('check_python_dependencies')
        
        this.pythonVersion = result.python_version
        this.dependenciesOk = result.dependencies_ok
        this.missingDependencies = result.missing_dependencies
        this.dependenciesChecked = true
        
        if (!result.dependencies_ok) {
          this.error = `Faltan dependencias Python:\n${result.missing_dependencies.join(', ')}`
        }
      } catch (error) {
        this.dependenciesChecked = true
        this.dependenciesOk = false
        this.error = 'Error verificando dependencias:\n' + error
      }
    },
    
    async selectFile() {
      try {
        const selected = await open({
          filters: [{
            name: 'Dump Files',
            extensions: ['dump']
          }],
          multiple: false
        })
        
        if (selected) {
          this.params.input_file = selected
          this.validationErrors.input_file = null
        }
      } catch (error) {
        this.error = 'Error al seleccionar archivo:\n' + error
      }
    },
    
    validateForm() {
      this.validationErrors = {}
      
      if (!this.params.input_file) {
        this.validationErrors.input_file = 'Debes seleccionar un archivo'
      }
      
      if (this.params.min_atoms < 1) {
        this.validationErrors.min_atoms = 'Debe ser mayor a 0'
      }
      
      return Object.keys(this.validationErrors).length === 0
    },
    
    async runClustering() {
      if (!this.validateForm()) {
        this.error = 'Por favor corrige los errores en el formulario'
        return
      }
      
      if (!this.dependenciesOk) {
        this.error = 'Debes instalar las dependencias Python requeridas'
        return
      }
      
      this.isProcessing = true
      this.error = null
      this.results = null
      this.progressLogs = []
      this.progressPercentage = 0

      try {
        const result = await invoke('run_clustering', { params: this.params })
        this.results = result
        
        if (result.success) {
          this.successMessage = `Clustering completado: ${result.clusters_found} clusters`
          this.progressPercentage = 100
          await this.saveCurrentConfig(true)
        } else {
          this.error = result.message
        }
      } catch (error) {
        this.error = 'Error durante el clustering:\n' + error
        this.progressPercentage = 0
      } finally {
        this.isProcessing = false
      }
    },
    
    async cancelProcess() {
      try {
        await invoke('cancel_clustering')
        this.addLog('⚠️ Cancelación solicitada...', 'warning')
      } catch (error) {
        this.error = 'Error al cancelar:\n' + error
      }
    },
    
    resetParams() {
      this.params = {
        input_file: '',
        output_dir: 'clusters_output',
        min_atoms: 50,
        max_iterations: 5,
        n_clusters_per_level: null,
        silhouette_threshold: 0.3,
        davies_bouldin_threshold: 1.5,
        dispersion_threshold: null,
        quantile: 0.2
      }
      this.results = null
      this.error = null
      this.validationErrors = {}
      this.progressLogs = []
      this.progressPercentage = 0
      this.successMessage = null
    },
    
    async saveCurrentConfig(silent = false) {
      try {
        await invoke('save_config', { params: this.params })
        if (!silent) {
          this.successMessage = '💾 Configuración guardada'
          setTimeout(() => this.successMessage = null, 3000)
        }
      } catch (error) {
        if (!silent) {
          this.error = 'Error guardando configuración:\n' + error
        }
      }
    },
    
    async loadSavedConfig() {
      try {
        const loadedParams = await invoke('load_config')
        this.params = loadedParams
        this.successMessage = '📂 Configuración cargada'
        setTimeout(() => this.successMessage = null, 3000)
      } catch (error) {
        this.error = 'No hay configuración guardada'
        setTimeout(() => this.error = null, 3000)
      }
    },
    
    async openResultsFolder() {
      try {
        await invoke('open_output_folder', { outputDir: this.params.output_dir })
      } catch (error) {
        this.error = 'Error abriendo carpeta:\n' + error
      }
    },
    
    addLog(message, type = 'info') {
      const now = new Date()
      const time = now.toLocaleTimeString('es-ES', { 
        hour: '2-digit', 
        minute: '2-digit', 
        second: '2-digit' 
      })
      
      this.progressLogs.push({ time, message, type })
      
      this.$nextTick(() => {
        const container = this.$refs.logsContainer
        if (container) {
          container.scrollTop = container.scrollHeight
        }
      })
    },
    
    getLogClass(log) {
      if (log.message.includes('✅') || log.message.includes('completado')) {
        return 'log-success'
      }
      if (log.message.includes('❌') || log.message.includes('Error')) {
        return 'log-error'
      }
      if (log.message.includes('⚠️')) {
        return 'log-warning'
      }
      return 'log-info'
    },
    
    clearLogs() {
      this.progressLogs = []
    }
  },
  async mounted() {
    await this.checkDependencies()
    
    this.unlistenProgress = await listen('clustering-progress', (event) => {
      const payload = event.payload
      this.addLog(payload.message)
      
      if (payload.progress !== null && payload.progress !== undefined) {
        this.progressPercentage = payload.progress
      }
    })
    
    try {
      const loadedParams = await invoke('load_config')
      this.params.output_dir = loadedParams.output_dir
      this.params.min_atoms = loadedParams.min_atoms
      this.params.max_iterations = loadedParams.max_iterations
      this.params.n_clusters_per_level = loadedParams.n_clusters_per_level
      this.params.silhouette_threshold = loadedParams.silhouette_threshold
      this.params.davies_bouldin_threshold = loadedParams.davies_bouldin_threshold
      this.params.dispersion_threshold = loadedParams.dispersion_threshold
      this.params.quantile = loadedParams.quantile
    } catch {
      // No hay config guardada
    }
  },
  beforeUnmount() {
    if (this.unlistenProgress) {
      this.unlistenProgress()
    }
  }
}


async function leerArchivoDump() {
  try {
    const contenido = await invoke("leer_dump");
    console.log("Contenido del dump:\n", contenido.slice(0, 200)); // muestra los primeros 200 caracteres
  } catch (error) {
    console.error("Error leyendo el dump:", error);
  }
}
</script>

<style>
/* (El CSS se mantiene igual, pero lo incluyo completo por si acaso) */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  min-height: 100vh;
}

#app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.app-header {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(20px);
  padding: 2rem;
  color: white;
}

.header-content {
  max-width: 1400px;
  margin: 0 auto;
}

.app-header h1 {
  font-size: 2.5rem;
  margin-bottom: 0.5rem;
  font-weight: 800;
}

.header-info {
  margin-top: 1rem;
}

.info-badge {
  display: inline-block;
  padding: 0.5rem 1rem;
  background: rgba(255, 255, 255, 0.2);
  border-radius: 20px;
  font-size: 0.9rem;
  font-weight: 600;
}

.badge-success {
  background: rgba(16, 185, 129, 0.3);
}

.badge-error {
  background: rgba(239, 68, 68, 0.3);
}

.app-main {
  flex: 1;
  padding: 2rem;
  max-width: 1400px;
  margin: 0 auto;
  width: 100%;
}

.card {
  background: white;
  border-radius: 15px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
  margin-bottom: 2rem;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1.5rem 2rem;
  border-bottom: 2px solid #e5e7eb;
}

.card-header h2 {
  font-size: 1.5rem;
  font-weight: 700;
  color: #333;
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 0.5rem;
}

.alert {
  padding: 1.5rem;
  border-radius: 12px;
  margin-bottom: 2rem;
}

.alert-error {
  background: #fef2f2;
  border: 2px solid #ef4444;
  color: #991b1b;
}

.alert-success {
  background: #f0fdf4;
  border: 2px solid #10b981;
  color: #065f46;
}

.alert-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 1rem;
}

.config-panel {
  padding: 2rem;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 1.5rem;
  margin-bottom: 2rem;
}

.span-2 {
  grid-column: span 2;
}

.form-group {
  display: flex;
  flex-direction: column;
}

.form-group label {
  font-weight: 600;
  margin-bottom: 0.5rem;
  color: #333;
}

.required {
  color: #ef4444;
}

.file-input-group {
  display: flex;
  gap: 0.5rem;
}

.file-input,
.param-input,
.param-select {
  flex: 1;
  padding: 0.75rem;
  border: 2px solid #e5e7eb;
  border-radius: 8px;
  font-size: 1rem;
  transition: all 0.2s;
}

.file-input:focus,
.param-input:focus,
.param-select:focus {
  outline: none;
  border-color: #667eea;
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
}

.input-error {
  border-color: #ef4444;
}

.error-text {
  color: #ef4444;
  font-size: 0.85rem;
  margin-top: 0.25rem;
}

.param-slider {
  width: 100%;
  height: 6px;
  border-radius: 3px;
  background: #e5e7eb;
  outline: none;
  -webkit-appearance: none;
}

.param-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #667eea;
  cursor: pointer;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.action-buttons {
  display: flex;
  gap: 1rem;
  justify-content: center;
  flex-wrap: wrap;
}

.btn-primary,
.btn-secondary,
.btn-danger,
.btn-icon {
  padding: 0.875rem 1.75rem;
  border: none;
  border-radius: 8px;
  font-size: 1rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
}

.btn-primary {
  background: linear-gradient(135deg, #667eea, #764ba2);
  color: white;
  min-width: 200px;
}

.btn-primary:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
}

.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-secondary {
  background: #f8f9fa;
  color: #333;
  border: 2px solid #e5e7eb;
}

.btn-danger {
  background: #ef4444;
  color: white;
}

.btn-icon {
  padding: 0.5rem;
  min-width: auto;
  background: transparent;
  border: 1px solid #e5e7eb;
  font-size: 1.2rem;
}

.btn-close {
  background: none;
  border: none;
  font-size: 1.5rem;
  cursor: pointer;
  padding: 0;
  width: 30px;
  height: 30px;
}

.spinner {
  display: inline-block;
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.progress-panel {
  padding: 1.5rem 2rem;
}

.progress-bar-container {
  width: 100%;
  height: 30px;
  background: #f8f9fa;
  border-radius: 15px;
  overflow: hidden;
  margin-bottom: 1rem;
}

.progress-bar {
  height: 100%;
  background: linear-gradient(135deg, #667eea, #764ba2);
  transition: width 0.5s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 50px;
}

.progress-text {
  color: white;
  font-weight: 700;
  font-size: 0.85rem;
}

.logs-container {
  max-height: 400px;
  overflow-y: auto;
  background: #1e293b;
  border-radius: 8px;
  padding: 1rem;
  font-family: 'Courier New', monospace;
  font-size: 0.85rem;
  color: #e2e8f0;
}

.log-line {
  padding: 0.4rem 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  display: flex;
  gap: 1rem;
}

.log-time {
  color: #94a3b8;
  flex-shrink: 0;
}

.log-success { color: #4ade80; }
.log-error { color: #f87171; }
.log-warning { color: #fbbf24; }
.log-info { color: #e2e8f0; }

.results-panel {
  padding: 2rem;
}

.results-summary {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 1.5rem;
  margin-bottom: 2rem;
}

.stat-card {
  background: linear-gradient(135deg, #667eea, #764ba2);
  color: white;
  padding: 1.5rem;
  border-radius: 12px;
  text-align: center;
}

.stat-value {
  font-size: 2rem;
  font-weight: 800;
  margin: 0;
}

.clusters-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.cluster-item {
  background: #f8f9fa;
  border: 2px solid #e5e7eb;
  border-radius: 8px;
  padding: 1.25rem;
  transition: all 0.2s;
}

.cluster-item:hover {
  border-color: #667eea;
  transform: translateX(4px);
}

.cluster-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 0.75rem;
}

.cluster-name {
  font-weight: 700;
  color: #333;
}

.cluster-atoms {
  background: #667eea;
  color: white;
  padding: 0.4rem 0.9rem;
  border-radius: 20px;
  font-size: 0.85rem;
  font-weight: 700;
}

.cluster-details {
  display: flex;
  gap: 1rem;
  font-size: 0.9rem;
  color: #666;
}

.cluster-level {
  background: #e5e7eb;
  padding: 0.3rem 0.7rem;
  border-radius: 6px;
}

.app-footer {
  background: rgba(0, 0, 0, 0.1);
  color: white;
  text-align: center;
  padding: 1.5rem;
}

@media (max-width: 768px) {
  .form-grid {
    grid-template-columns: 1fr;
  }
  .span-2 {
    grid-column: span 1;
  }
}
</style>